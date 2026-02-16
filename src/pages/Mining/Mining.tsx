import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Display, DisplayTitle, MainContainer, Dots } from 'src/components';
import Pill from 'src/components/Pill/Pill';
import useQueryContract from 'src/hooks/contract/useQueryContract';
import { useSigningClient } from 'src/contexts/signerClient';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppSelector } from 'src/redux/hooks';
import { UHASH_CONTRACT, UHASH_RELAY_URL } from 'src/constants/mining';
import { isTauri } from 'src/utils/tauri';
import { trimString, formatNumber } from 'src/utils/utils';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';
import type {
  RelayProofRequest,
  RelayProofResponse,
  SubmitErrorKind,
  SubmitProofMsg,
} from 'src/types/miningProofTx';
import useLiBalance from './hooks/useLiBalance';
import useRewardEstimate from './hooks/useRewardEstimate';
import useHashrateSamples from './hooks/useHashrateSamples';
import useMinerStats from './hooks/useMinerStats';
import HashrateHero from './components/HashrateHero';
import StatCard from './components/StatCard';
import ProofLogEntry from './components/ProofLogEntry';
import ThreadSelector from './components/ThreadSelector';
import MiningActionBar from './MiningActionBar';
import { WasmMiner } from './wasmMiner';
import styles from './Mining.module.scss';

type MiningStatus = {
  mining: boolean;
  hashrate: number;
  total_hashes: number;
  elapsed_secs: number;
  pending_proofs: number;
};

type ProofLogEntry_ = {
  hash: string;
  nonce: number;
  txHash?: string;
  error?: string;
  timestamp: number;
};

const PROOF_LOG_KEY = 'mining_proof_log';
const SESSION_LI_KEY = 'mining_session_li';

function loadProofLog(): ProofLogEntry_[] {
  try {
    const raw = localStorage.getItem(PROOF_LOG_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

function saveProofLog(log: ProofLogEntry_[]) {
  try {
    localStorage.setItem(PROOF_LOG_KEY, JSON.stringify(log.slice(0, 20)));
  } catch {
    // ignore
  }
}

function loadSessionLi(): number {
  try {
    return Number(localStorage.getItem(SESSION_LI_KEY)) || 0;
  } catch {
    return 0;
  }
}

function formatElapsed(seconds: number): string {
  if (seconds < 60) {
    return `${seconds.toFixed(0)}s`;
  }
  if (seconds < 3600) {
    return `${Math.floor(seconds / 60)}m ${Math.floor(seconds % 60)}s`;
  }
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
}

type Proof = { hash: string; nonce: number; timestamp: number };

// Min seconds between submissions (roughly one Bostrom block)
const SUBMIT_COOLDOWN_MS = 6_000;

function normalizeErrorText(error: unknown): string {
  if (!error) {
    return '';
  }
  if (typeof error === 'string') {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

function classifySubmitError(error: unknown): SubmitErrorKind {
  const anyError = error as
    | { code?: number; message?: string; rawLog?: string }
    | undefined;
  if (anyError?.code === 5) {
    return 'account_not_found';
  }

  const message = normalizeErrorText(error).toLowerCase();
  if (
    /does not exist on chain|account .*not found|unknown address|code\\s*[:=]\\s*5/.test(
      message
    )
  ) {
    return 'account_not_found';
  }
  if (
    /network|fetch|timeout|timed out|connection|econn|socket|dns|unavailable|503|502/.test(
      message
    )
  ) {
    return 'transport';
  }
  if (
    /failed to execute|codespace|wasm|out of gas|unauthorized|insufficient/.test(
      message
    )
  ) {
    return 'contract';
  }

  return 'unknown';
}

async function relayProof(
  proof: Proof,
  minerAddress: string
): Promise<string | null> {
  try {
    const payload: RelayProofRequest = {
      hash: proof.hash,
      nonce: Number(proof.nonce),
      timestamp: Number(proof.timestamp),
      miner_address: minerAddress,
    };
    const res = await fetch(UHASH_RELAY_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    });
    const data = (await res.json()) as RelayProofResponse;
    console.log('[Mining] Relay result:', data);
    return data.ok ? data.tx_hash : null;
  } catch (err) {
    console.error('[Mining] Relay failed:', err);
    return null;
  }
}

function Mining() {
  const address = useAppSelector(selectCurrentAddress);
  const { signer, signingClient } = useSigningClient();

  const { data: seedData } = useQueryContract(UHASH_CONTRACT, { seed: {} });
  const { data: difficultyData } = useQueryContract(UHASH_CONTRACT, {
    difficulty: {},
  });

  const seed = (seedData as any)?.seed as string | undefined;
  const difficulty = (difficultyData as any)?.current as number | undefined;

  const [miningStatus, setMiningStatus] = useState<MiningStatus | null>(null);
  const [autoMining, setAutoMining] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [proofLog, setProofLog] = useState<ProofLogEntry_[]>(loadProofLog);
  const [threadCount, setThreadCount] = useState(() =>
    Math.max(1, (navigator.hardwareConcurrency || 4) - 1)
  );
  const [sessionLiMined, setSessionLiMined] = useState(loadSessionLi);

  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const autoMiningRef = useRef(false);
  const wasmMinerRef = useRef<WasmMiner | null>(null);
  const isNative = isTauri();

  // Proof submission queue: accumulates proofs, submits best one at a time
  const proofQueueRef = useRef<Proof[]>([]);
  const submittingRef = useRef(false);
  const lastSubmitTimeRef = useRef(0);

  const hashrate = miningStatus?.hashrate ?? 0;
  const elapsed = miningStatus?.elapsed_secs ?? 0;

  const { balance: liBalance, refetch: refreshBalance } = useLiBalance(address);
  const { rewardPerProof, estimatedLiPerHour } = useRewardEstimate(
    difficulty,
    hashrate
  );
  const samples = useHashrateSamples(hashrate, autoMining);
  const { uniqueMiners } = useMinerStats();

  // Keep autoMining ref in sync with state
  useEffect(() => {
    autoMiningRef.current = autoMining;
  }, [autoMining]);

  // Persist proof log and session LI to localStorage
  useEffect(() => {
    saveProofLog(proofLog);
  }, [proofLog]);

  useEffect(() => {
    try {
      localStorage.setItem(SESSION_LI_KEY, String(sessionLiMined));
    } catch {
      // ignore
    }
  }, [sessionLiMined]);

  const stopPolling = useCallback(() => {
    if (pollRef.current) {
      clearInterval(pollRef.current);
      pollRef.current = null;
    }
  }, []);

  const startMiningRound = useCallback(async () => {
    if (!seed || !difficulty || !address) {
      console.log('[Mining] Cannot start: missing seed/difficulty/address');
      return;
    }

    try {
      const timestamp = Math.floor(Date.now() / 1000);
      console.log('[Mining] Starting mining round, difficulty:', difficulty);

      if (isNative) {
        await invoke('start_mining', {
          seed,
          address,
          timestamp,
          difficulty,
          threads: threadCount,
        });
      } else {
        if (!wasmMinerRef.current) {
          const miner = new WasmMiner(threadCount);
          await miner.init();
          wasmMinerRef.current = miner;
        }
        wasmMinerRef.current.start(seed, address, timestamp, difficulty);
      }
    } catch (err) {
      console.error('[Mining] Failed to start mining', err);
    }
  }, [seed, difficulty, address, threadCount, isNative]);

  // Submit a single proof to chain — handles new-account fallback
  const submitSingleProof = useCallback(
    async (proof: Proof) => {
      if (!signer || !signingClient || !address) {
        console.log('[Mining] Cannot submit: no signer/client');
        return;
      }

      const [account] = await signer.getAccounts();
      const msg: SubmitProofMsg = {
        submit_proof: {
          hash: proof.hash,
          nonce: Number(proof.nonce),
          timestamp: Number(proof.timestamp),
        },
      };

      console.log(
        '[Mining] Submitting proof:',
        `${proof.hash.slice(0, 16)}...`
      );

      let result;
      try {
        result = await signingClient.execute(
          account.address,
          UHASH_CONTRACT,
          msg,
          Soft3MessageFactory.fee(8),
          ''
        );
      } catch (executeErr: any) {
        const kind = classifySubmitError(executeErr);

        // New accounts don't exist on-chain yet — relay the proof instead
        if (kind === 'account_not_found') {
          console.log('[Mining] Account not on chain, relaying proof...');
          const txHash = await relayProof(proof, account.address);
          if (txHash) {
            console.log('[Mining] Proof relayed! TX:', txHash);
            // Wait for relay TX to be included (creates account via LI mint)
            await new Promise<void>((resolve) => {
              setTimeout(resolve, 7000);
            });
            // Return early — proof was already submitted via relay
            setProofLog((prev) => [
              {
                hash: proof.hash,
                nonce: proof.nonce,
                txHash,
                timestamp: Date.now(),
              },
              ...prev,
            ]);
            refreshBalance();
            setSessionLiMined((prev) => prev + (rewardPerProof || 0));
            return;
          }
          throw new Error('Proof relay failed — account does not exist');
        }
        if (kind === 'transport') {
          throw new Error(
            `Submit transport error: ${normalizeErrorText(executeErr)}`
          );
        }
        throw new Error(
          `Submit ${kind} error: ${normalizeErrorText(executeErr)}`
        );
      }

      console.log(
        '[Mining] Proof submitted! TX:',
        result.transactionHash,
        'events:',
        result.events?.length,
        'types:',
        result.events?.map((e: any) => e.type)
      );

      // Extract actual reward from wasm event
      let actualReward = 0;
      if (result.events) {
        const wasmEvent = result.events.find(
          (e: { type: string }) => e.type === 'wasm'
        );
        if (wasmEvent) {
          console.log(
            '[Mining] wasm attrs:',
            wasmEvent.attributes?.map((a: any) => `${a.key}=${a.value}`)
          );
          const rewardAttr = wasmEvent.attributes?.find(
            (a: { key: string }) => a.key === 'reward'
          );
          if (rewardAttr?.value) {
            actualReward = Number(rewardAttr.value) / 1_000_000;
          }
        }
      }
      // Fallback: use contract estimate if events didn't have reward
      if (actualReward === 0) {
        actualReward = rewardPerProof || 0;
      }
      console.log('[Mining] reward:', actualReward);

      setProofLog((prev) => [
        {
          hash: proof.hash,
          nonce: proof.nonce,
          txHash: result.transactionHash,
          timestamp: Date.now(),
        },
        ...prev,
      ]);
      refreshBalance();
      setSessionLiMined((prev) => prev + actualReward);
    },
    [signer, signingClient, address, refreshBalance, rewardPerProof]
  );

  // Process the proof queue: pick best proof, wait for cooldown, submit sequentially
  const processQueue = useCallback(async () => {
    if (submittingRef.current) {
      return;
    } // already processing
    if (proofQueueRef.current.length === 0) {
      return;
    }

    // Enforce cooldown between submissions (one per block)
    const now = Date.now();
    const elapsed_ = now - lastSubmitTimeRef.current;
    if (elapsed_ < SUBMIT_COOLDOWN_MS) {
      console.log(
        `[Mining] Cooldown: ${((SUBMIT_COOLDOWN_MS - elapsed_) / 1000).toFixed(
          1
        )}s remaining`
      );
      return;
    }

    submittingRef.current = true;
    setSubmitting(true);

    // Pick the best proof (lowest hash = most work = highest reward)
    const queue = proofQueueRef.current;
    let bestIdx = 0;
    for (let i = 1; i < queue.length; i++) {
      if (queue[i].hash < queue[bestIdx].hash) {
        bestIdx = i;
      }
    }
    const best = queue[bestIdx];

    // Clear the queue — discard inferior proofs
    const discarded = queue.length - 1;
    proofQueueRef.current = [];
    if (discarded > 0) {
      console.log(
        `[Mining] Submitting best of ${
          discarded + 1
        } proofs, discarded ${discarded}`
      );
    }

    try {
      await submitSingleProof(best);
      lastSubmitTimeRef.current = Date.now();
    } catch (err: any) {
      console.error('[Mining] Submit failed:', err);
      setProofLog((prev) => [
        {
          hash: best.hash,
          nonce: best.nonce,
          error: err?.message || 'Failed',
          timestamp: Date.now(),
        },
        ...prev,
      ]);
    } finally {
      submittingRef.current = false;
      setSubmitting(false);
    }
  }, [submitSingleProof]);

  // Poll mining status and enqueue proofs for sequential submission
  const startPolling = useCallback(() => {
    stopPolling();
    pollRef.current = setInterval(async () => {
      try {
        let status: MiningStatus;
        if (isNative) {
          status = (await invoke('get_mining_status')) as MiningStatus;
        } else if (wasmMinerRef.current) {
          status = wasmMinerRef.current.getStatus();
        } else {
          return;
        }
        setMiningStatus(status);

        // Drain new proofs into the queue
        if (status.pending_proofs > 0 && autoMiningRef.current) {
          let proofs: Proof[];
          if (isNative) {
            proofs = (await invoke('take_proofs')) as Proof[];
          } else if (wasmMinerRef.current) {
            proofs = wasmMinerRef.current.takeProofs();
          } else {
            proofs = [];
          }

          if (proofs.length > 0) {
            proofQueueRef.current.push(...proofs);
            console.log(
              `[Mining] ${proofs.length} proof(s) queued, total pending: ${proofQueueRef.current.length}`
            );
          }
        }

        // Try to process the queue (respects cooldown + sequential lock)
        processQueue();
      } catch (err) {
        console.error('[Mining] Poll error', err);
      }
    }, 500);
  }, [stopPolling, processQueue, isNative]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      stopPolling();
      if (!isNative && wasmMinerRef.current) {
        wasmMinerRef.current.destroy();
        wasmMinerRef.current = null;
      }
    };
  }, [stopPolling, isNative]);

  // On mount: detect if mining is still active in the backend and resume UI (Tauri only)
  useEffect(() => {
    if (!isNative) {
      return undefined;
    }
    let cancelled = false;
    (async () => {
      try {
        const status = (await invoke('get_mining_status')) as MiningStatus;
        if (cancelled) {
          return;
        }
        if (status.mining) {
          console.log('[Mining] Detected active mining on mount, resuming UI');
          setMiningStatus(status);
          setAutoMining(true);
          startPolling();
        }
      } catch {
        // not in Tauri or mining not initialized
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleStartMining = useCallback(async () => {
    setAutoMining(true);
    setSessionLiMined(0);
    await startMiningRound();
    startPolling();
  }, [startMiningRound, startPolling]);

  const handleStopMining = useCallback(async () => {
    setAutoMining(false);
    try {
      if (isNative) {
        await invoke('stop_mining');
        const status = (await invoke('get_mining_status')) as MiningStatus;
        setMiningStatus(status);
      } else if (wasmMinerRef.current) {
        wasmMinerRef.current.stop();
        setMiningStatus(wasmMinerRef.current.getStatus());
      }
    } catch (err) {
      console.error('[Mining] Failed to stop mining', err);
    }
    stopPolling();
  }, [stopPolling, isNative]);

  const handleCopyAddress = useCallback(() => {
    if (address) {
      navigator.clipboard.writeText(address);
    }
  }, [address]);

  return (
    <MainContainer>
      <Display title={<DisplayTitle title="Mining" />}>
        <div className={styles.wrapper}>
          {/* Header: wallet + status */}
          <div className={styles.header}>
            <div className={styles.walletInfo}>
              {address ? trimString(address, 12, 6) : 'No wallet'}
              {address && (
                <button
                  type="button"
                  className={styles.copyBtn}
                  onClick={handleCopyAddress}
                  title="Copy address"
                >
                  copy
                </button>
              )}
            </div>
            <Pill
              color={autoMining ? 'green' : 'black'}
              text={autoMining ? 'Mining' : 'Idle'}
            />
          </div>

          {/* Hero: big hashrate + sparkline */}
          <HashrateHero
            hashrate={hashrate}
            isActive={autoMining}
            samples={samples}
          />

          {/* 4-card stat grid */}
          <div className={styles.statsGrid}>
            <StatCard
              label="LI Mined"
              value={
                sessionLiMined < 0.1
                  ? sessionLiMined.toFixed(4)
                  : sessionLiMined.toFixed(2)
              }
              suffix="LI"
            />
            <StatCard
              label="Proofs"
              value={proofLog.filter((p) => p.txHash).length}
            />
            <StatCard
              label="Est. LI/hr"
              value={`~${
                estimatedLiPerHour < 1
                  ? estimatedLiPerHour.toFixed(2)
                  : estimatedLiPerHour.toFixed(0)
              }`}
            />
            <StatCard label="Elapsed" value={formatElapsed(elapsed)} />
          </div>

          {/* LI Balance row */}
          <div className={styles.balanceRow}>
            <span>LI Balance</span>
            <span>{formatNumber(liBalance)} LI</span>
          </div>

          {/* Network info + thread selector */}
          <div className={styles.networkRow}>
            <div>
              Difficulty: {difficulty ?? '...'} · Miners: {uniqueMiners}
            </div>
            <ThreadSelector
              value={threadCount}
              onChange={setThreadCount}
              max={navigator.hardwareConcurrency || 4}
              disabled={autoMining}
            />
          </div>

          {/* Submitting indicator */}
          {submitting && (
            <div className={styles.submitting}>
              Submitting proof
              <Dots />
            </div>
          )}

          {/* Proof log */}
          {proofLog.length > 0 && (
            <div className={styles.proofLog}>
              <span className={styles.proofLogTitle}>Recent Proofs</span>
              {proofLog.map((entry, i) => (
                <ProofLogEntry
                  key={`${entry.timestamp}-${i}`}
                  index={proofLog.length - i}
                  hash={entry.hash}
                  txHash={entry.txHash}
                  error={entry.error}
                  timestamp={entry.timestamp}
                />
              ))}
            </div>
          )}
        </div>
      </Display>

      <MiningActionBar
        seed={seed}
        difficulty={difficulty}
        address={address}
        autoMining={autoMining}
        submitting={submitting}
        miningStatus={miningStatus}
        onStartMining={handleStartMining}
        onStopMining={handleStopMining}
      />
    </MainContainer>
  );
}

export default Mining;

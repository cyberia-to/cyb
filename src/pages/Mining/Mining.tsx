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
  LithiumEpochStatus,
  LithiumMinerEpochStatsResponse,
  LithiumProofStatsResponse,
  LithiumTargetResponse,
  RelayProofRequest,
  RelayProofResponse,
  SubmitErrorKind,
  SubmitLithiumProofMsg,
} from 'src/types/miningProofTx';
import useLiBalance from './hooks/useLiBalance';
import useRewardEstimate from './hooks/useRewardEstimate';
import useHashrateSamples from './hooks/useHashrateSamples';
import useMinerStats from './hooks/useMinerStats';
import usePeerEstimate from './hooks/usePeerEstimate';
import useLatestBlock from './hooks/useLatestBlock';
import useEmissionInfo from './hooks/useEmissionInfo';
import useBurnStats from './hooks/useBurnStats';
import HashrateHero from './components/HashrateHero';
import StatCard from './components/StatCard';
import ProofLogEntry from './components/ProofLogEntry';
import ThreadSelector from './components/ThreadSelector';
import StakingSection from './components/StakingSection';
import ReferralSection, { loadReferrer } from './components/ReferralSection';
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

type Proof = { hash: string; nonce: number };

// Min seconds between submissions (roughly one Bostrom block)
const SUBMIT_COOLDOWN_MS = 6_000;

function formatHashrate(hps: number): string {
  if (hps >= 1_000_000) return `${(hps / 1_000_000).toFixed(1)} MH/s`;
  if (hps >= 1_000) return `${(hps / 1_000).toFixed(1)} KH/s`;
  return `${hps.toFixed(0)} H/s`;
}

function formatLi(amount: string | undefined): string {
  if (!amount) return '0';
  const val = Number(amount) / 1_000_000;
  return val < 0.01 ? val.toFixed(4) : formatNumber(val);
}

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
    /does not exist on chain|account .*not found|unknown address|code\s*[:=]\s*5/.test(
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
  minerAddress: string,
  blockHash: string,
  dataHash: string,
  epochId: number,
  referrer: string | undefined
): Promise<string | null> {
  try {
    const payload: RelayProofRequest = {
      hash: proof.hash,
      nonce: Number(proof.nonce),
      miner_address: minerAddress,
      block_hash: blockHash,
      cyberlinks_merkle: dataHash,
      epoch_id: epochId,
      timestamp: Math.floor(Date.now() / 1000),
      referrer: referrer || undefined,
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

  const { data: epochData } = useQueryContract(UHASH_CONTRACT, {
    epoch_status: {},
  });
  const { data: difficultyData } = useQueryContract(UHASH_CONTRACT, {
    difficulty: {},
  });
  const { data: targetData } = useQueryContract(UHASH_CONTRACT, {
    target: {},
  });
  const { data: proofStatsData } = useQueryContract(UHASH_CONTRACT, {
    proof_stats: {},
  });

  const difficulty = (difficultyData as any)?.current as number | undefined;
  const epochStatus = epochData as LithiumEpochStatus | undefined;
  const targetStats = targetData as LithiumTargetResponse | undefined;
  const proofStats = proofStatsData as LithiumProofStatsResponse | undefined;
  const epochId = epochStatus?.epoch_id ?? proofStats?.epoch_id;
  const targetSolutions =
    targetStats?.target_solutions ?? epochStatus?.target_solutions;

  const { data: minerEpochData } = useQueryContract(
    UHASH_CONTRACT,
    address && epochId !== undefined
      ? { lithium_miner_epoch_stats: { address, epoch_id: epochId } }
      : { epoch_status: {} }
  );
  const minerEpochStats = minerEpochData as
    | LithiumMinerEpochStatsResponse
    | undefined;
  const minerEpochProofCount =
    address && epochId !== undefined ? minerEpochStats?.proof_count ?? 0 : 0;

  const latestBlock = useLatestBlock();
  const { emission } = useEmissionInfo();
  const { burnStats } = useBurnStats();

  const [miningStatus, setMiningStatus] = useState<MiningStatus | null>(null);
  const [autoMining, setAutoMining] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [proofLog, setProofLog] = useState<ProofLogEntry_[]>(loadProofLog);
  const [threadCount, setThreadCount] = useState(() =>
    Math.max(1, (navigator.hardwareConcurrency || 4) - 1)
  );
  const [sessionLiMined, setSessionLiMined] = useState(loadSessionLi);
  const [referrer, setReferrer] = useState(() => loadReferrer());

  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const autoMiningRef = useRef(false);
  const wasmMinerRef = useRef<WasmMiner | null>(null);
  const isNative = isTauri();

  // Track current block/epoch so proof submission uses the values from when mining started
  const blockHashRef = useRef<string>('');
  const dataHashRef = useRef<string>('');
  const epochIdRef = useRef<number>(0);

  // Proof submission queue
  const proofQueueRef = useRef<Proof[]>([]);
  const submittingRef = useRef(false);
  const lastSubmitTimeRef = useRef(0);

  const hashrate = miningStatus?.hashrate ?? 0;
  const elapsed = miningStatus?.elapsed_secs ?? 0;

  const { balance: liBalance, refetch: refreshBalance } = useLiBalance(address);
  const { rewardPerProof, grossRewardPerProof, miningFraction, estimatedLiPerHour } =
    useRewardEstimate(difficulty, hashrate);
  const samples = useHashrateSamples(hashrate, autoMining);
  const { uniqueMiners } = useMinerStats();
  const { networkHashrate, similarDevices } = usePeerEstimate(hashrate);

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
    if (!difficulty || !address || !latestBlock) {
      console.log('[Mining] Cannot start: missing difficulty/address/block');
      return;
    }

    const { blockHash, dataHash } = latestBlock;
    blockHashRef.current = blockHash;
    dataHashRef.current = dataHash;
    epochIdRef.current = epochId ?? 0;

    try {
      console.log(
        '[Mining] Starting lithium mining, difficulty:',
        difficulty,
        'block:',
        blockHash.slice(0, 16)
      );

      if (isNative) {
        await invoke('start_mining', {
          address,
          blockHashHex: blockHash,
          cyberlinksMerkleHex: dataHash,
          difficulty,
          threads: threadCount,
        });
      } else {
        if (!wasmMinerRef.current) {
          const miner = new WasmMiner(threadCount);
          await miner.init();
          wasmMinerRef.current = miner;
        }
        wasmMinerRef.current.start(address, blockHash, dataHash, difficulty);
      }
    } catch (err) {
      console.error('[Mining] Failed to start mining', err);
    }
  }, [difficulty, address, latestBlock, epochId, threadCount, isNative]);

  // Submit a single proof to chain
  const submitSingleProof = useCallback(
    async (proof: Proof) => {
      if (!signer || !signingClient || !address) {
        console.log('[Mining] Cannot submit: no signer/client');
        return;
      }

      const [account] = await signer.getAccounts();
      const msg: SubmitLithiumProofMsg = {
        submit_lithium_proof: {
          hash: proof.hash,
          nonce: Number(proof.nonce),
          miner_address: address,
          block_hash: blockHashRef.current,
          cyberlinks_merkle: dataHashRef.current,
          epoch_id: epochIdRef.current,
          timestamp: Math.floor(Date.now() / 1000),
          referrer: referrer || undefined,
        },
      };

      console.log(
        '[Mining] Submitting lithium proof:',
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

        if (kind === 'account_not_found') {
          console.log('[Mining] Account not on chain, relaying proof...');
          const txHash = await relayProof(
            proof,
            account.address,
            blockHashRef.current,
            dataHashRef.current,
            epochIdRef.current,
            referrer
          );
          if (txHash) {
            console.log('[Mining] Proof relayed! TX:', txHash);
            await new Promise<void>((resolve) => {
              setTimeout(resolve, 7000);
            });
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
        result.events?.length
      );

      // Extract actual miner reward from wasm event
      let actualReward = 0;
      if (result.events) {
        const wasmEvent = result.events.find(
          (e: { type: string }) => e.type === 'wasm'
        );
        if (wasmEvent) {
          const rewardAttr = wasmEvent.attributes?.find(
            (a: { key: string }) =>
              a.key === 'miner_reward' || a.key === 'reward'
          );
          if (rewardAttr?.value) {
            actualReward = Number(rewardAttr.value) / 1_000_000;
          }
        }
      }
      if (actualReward === 0) {
        actualReward = rewardPerProof || 0;
      }

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
    [signer, signingClient, address, referrer, refreshBalance, rewardPerProof]
  );

  // Process the proof queue
  const processQueue = useCallback(async () => {
    if (submittingRef.current) {
      return;
    }
    if (proofQueueRef.current.length === 0) {
      return;
    }

    const now = Date.now();
    const elapsed_ = now - lastSubmitTimeRef.current;
    if (elapsed_ < SUBMIT_COOLDOWN_MS) {
      return;
    }

    submittingRef.current = true;
    setSubmitting(true);

    const queue = proofQueueRef.current;
    let bestIdx = 0;
    for (let i = 1; i < queue.length; i++) {
      if (queue[i].hash < queue[bestIdx].hash) {
        bestIdx = i;
      }
    }
    const best = queue[bestIdx];

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

  // Poll mining status and enqueue proofs
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

  // On mount: detect if mining is still active in the backend (Tauri only)
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

  const canMine = !!difficulty && !!address && !!latestBlock;

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

          {/* Emission info */}
          {emission && (
            <div className={styles.sectionBox}>
              <span className={styles.sectionTitle}>Emission (per window)</span>
              <div className={styles.statsGrid}>
                <StatCard
                  label="Mining"
                  value={formatLi(emission.mining_emission)}
                  suffix="LI"
                />
                <StatCard
                  label="Staking"
                  value={formatLi(emission.staking_emission)}
                  suffix="LI"
                />
                <StatCard
                  label="Referral"
                  value={formatLi(emission.referral_emission)}
                  suffix="LI"
                />
              </div>
            </div>
          )}

          {/* Burn stats */}
          {burnStats && (
            <div className={styles.balanceRow}>
              <span>Total Burned</span>
              <span>{formatLi(burnStats.total_burned)} LI</span>
            </div>
          )}

          {/* Network info + thread selector */}
          <div className={styles.networkRow}>
            <div>
              Difficulty: {difficulty ?? '...'} · Epoch: {epochId ?? '...'} ·
              Proofs: {proofStats?.proof_count ?? '...'}/
              {targetSolutions ?? '...'} · My proofs: {minerEpochProofCount} ·
              Net: {formatHashrate(networkHashrate)} ·{' '}
              {similarDevices > 0
                ? `~${similarDevices} similar devices`
                : '\u2014'}{' '}
              · {uniqueMiners} all-time miners
              {latestBlock && (
                <> · Block: {latestBlock.height}</>
              )}
            </div>
            <ThreadSelector
              value={threadCount}
              onChange={setThreadCount}
              max={navigator.hardwareConcurrency || 4}
              disabled={autoMining}
            />
          </div>

          {/* Staking section */}
          <StakingSection />

          {/* Referral section */}
          <ReferralSection
            referrer={referrer}
            onReferrerChange={setReferrer}
          />

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
        difficulty={difficulty}
        address={address}
        blockReady={!!latestBlock}
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

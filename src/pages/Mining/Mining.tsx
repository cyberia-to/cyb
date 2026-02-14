import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Display, DisplayTitle, MainContainer, Dots } from 'src/components';
import Pill from 'src/components/Pill/Pill';
import useQueryContract from 'src/hooks/contract/useQueryContract';
import { useSigningClient } from 'src/contexts/signerClient';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppSelector } from 'src/redux/hooks';
import { UHASH_CONTRACT } from 'src/constants/mining';
import { isTauri } from 'src/utils/tauri';
import { trimString, formatNumber } from 'src/utils/utils';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';
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
  if (seconds < 60) return `${seconds.toFixed(0)}s`;
  if (seconds < 3600)
    return `${Math.floor(seconds / 60)}m ${Math.floor(seconds % 60)}s`;
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
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

  // Submit proof to chain (fire-and-forget, mining continues)
  const submitProof = useCallback(
    async (proof: { hash: string; nonce: number; timestamp: number }) => {
      if (!signer || !signingClient || !address) {
        console.log('[Mining] Cannot submit: no signer/client');
        return;
      }

      setSubmitting(true);
      try {
        const [account] = await signer.getAccounts();
        const msg = {
          submit_proof: {
            hash: proof.hash,
            nonce: proof.nonce,
            timestamp: proof.timestamp,
          },
        };

        console.log(
          '[Mining] Submitting proof:',
          proof.hash.slice(0, 16) + '...'
        );
        const result = await signingClient.execute(
          account.address,
          UHASH_CONTRACT,
          msg,
          Soft3MessageFactory.fee(8),
          ''
        );

        console.log('[Mining] Proof submitted! TX:', result.transactionHash);
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
        setSessionLiMined((prev) => prev + rewardPerProof);
      } catch (err: any) {
        console.error('[Mining] Submit failed:', err);
        setProofLog((prev) => [
          {
            hash: proof.hash,
            nonce: proof.nonce,
            error: err?.message || 'Failed',
            timestamp: Date.now(),
          },
          ...prev,
        ]);
      } finally {
        setSubmitting(false);
      }
    },
    [signer, signingClient, address, refreshBalance, rewardPerProof]
  );

  // Poll mining status and drain proof queue
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

        // Drain and submit any pending proofs (async, non-blocking)
        if (status.pending_proofs > 0 && autoMiningRef.current) {
          let proofs: { hash: string; nonce: number; timestamp: number }[];
          if (isNative) {
            proofs = (await invoke('take_proofs')) as typeof proofs;
          } else if (wasmMinerRef.current) {
            proofs = wasmMinerRef.current.takeProofs();
          } else {
            proofs = [];
          }

          for (const proof of proofs) {
            console.log('[Mining] Proof found, submitting async...');
            submitProof(proof); // fire-and-forget, mining continues
          }
        }
      } catch (err) {
        console.error('[Mining] Poll error', err);
      }
    }, 500);
  }, [stopPolling, submitProof, isNative]);

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
    if (!isNative) return;
    let cancelled = false;
    (async () => {
      try {
        const status = (await invoke('get_mining_status')) as MiningStatus;
        if (cancelled) return;
        if (status.mining || status.hashrate > 0) {
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
              value={sessionLiMined.toFixed(1)}
              suffix="LI"
            />
            <StatCard
              label="Proofs"
              value={proofLog.filter((p) => p.txHash).length}
            />
            <StatCard
              label="Est. LI/hr"
              value={`~${estimatedLiPerHour < 1 ? estimatedLiPerHour.toFixed(2) : estimatedLiPerHour.toFixed(0)}`}
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
              {proofLog.slice(0, 5).map((entry, i) => (
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

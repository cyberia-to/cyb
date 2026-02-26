import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Display, DisplayTitle, MainContainer, Dots } from 'src/components';
import Pill from 'src/components/Pill/Pill';
import useQueryContract from 'src/hooks/contract/useQueryContract';
import { LITIUM_MINE_CONTRACT, UHASH_RELAY_URL, SUBMIT_COOLDOWN_MS } from 'src/constants/mining';
import { isTauri } from 'src/utils/tauri';
import { trimString } from 'src/utils/utils';
import { compactLi, formatLi } from './utils/formatLi';
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
import useAutoSigner from './hooks/useAutoSigner';
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
import ReferralSection, { loadReferrer, saveReferrer } from './components/ReferralSection';
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
const MINING_ACTIVE_KEY = 'mining_active';
const MINING_ADDRESS_KEY = 'mining_active_address';

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

function formatHashrate(hps: number): string {
  if (hps >= 1_000_000) return `${(hps / 1_000_000).toFixed(1)} MH/s`;
  if (hps >= 1_000) return `${(hps / 1_000).toFixed(1)} KH/s`;
  return `${hps.toFixed(0)} H/s`;
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
  referrer: string | undefined,
  blockTimestamp: number
): Promise<string | null> {
  try {
    const payload: RelayProofRequest = {
      hash: proof.hash,
      nonce: Number(proof.nonce),
      miner_address: minerAddress,
      block_hash: blockHash,
      cyberlinks_merkle: dataHash,
      epoch_id: epochId,
      timestamp: blockTimestamp,
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
  const { signer, signingClient, address } = useAutoSigner();

  const { data: epochData } = useQueryContract(LITIUM_MINE_CONTRACT, {
    epoch_status: {},
  });
  const { data: difficultyData } = useQueryContract(LITIUM_MINE_CONTRACT, {
    difficulty: {},
  });
  const { data: targetData } = useQueryContract(LITIUM_MINE_CONTRACT, {
    target: {},
  });
  const { data: proofStatsData } = useQueryContract(LITIUM_MINE_CONTRACT, {
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
    LITIUM_MINE_CONTRACT,
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
  const [referrer, setReferrer] = useState(() => {
    // Check URL ?ref= param first, then localStorage
    const params = new URLSearchParams(window.location.search);
    const refParam = params.get('ref');
    if (refParam) {
      saveReferrer(refParam);
      return refParam;
    }
    return loadReferrer();
  });

  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const autoMiningRef = useRef(false);
  const miningAddressRef = useRef<string | undefined>(undefined);
  const wasmMinerRef = useRef<WasmMiner | null>(null);
  const isNative = isTauri();

  // Track current block/epoch so proof submission uses the values from when mining started
  const blockHashRef = useRef<string>('');
  const dataHashRef = useRef<string>('');
  const epochIdRef = useRef<number>(0);
  const blockTimestampRef = useRef<number>(0);

  // Proof submission queue
  const proofQueueRef = useRef<Proof[]>([]);
  const submittingRef = useRef(false);
  const lastSubmitTimeRef = useRef(0);
  // Rolling hashrate snapshots for Tauri backend (WASM handles this internally)
  const hashSnapshotsRef = useRef<{ time: number; hashes: number }[]>([]);

  const hashrate = miningStatus?.hashrate ?? 0;
  const elapsed = miningStatus?.elapsed_secs ?? 0;
  const canMine = !!difficulty && !!address && !!latestBlock;

  const { balance: liBalance, refetch: refreshBalance } = useLiBalance(address);
  const { rewardPerProof, grossRewardPerProof, miningFraction, estimatedLiPerHour } =
    useRewardEstimate(difficulty, hashrate);
  const samples = useHashrateSamples(hashrate, autoMining);
  const { uniqueMiners, totalProofs, avgDifficulty } = useMinerStats();
  const { networkHashrate, similarDevices, minProfitable, dataUpdatedAt } = usePeerEstimate(hashrate);

  // Countdown to next contract data refresh (15s refetch interval)
  const [refreshCountdown, setRefreshCountdown] = useState(15);
  useEffect(() => {
    if (!dataUpdatedAt) return;
    const tick = () => {
      const elapsed_ = (Date.now() - dataUpdatedAt) / 1000;
      setRefreshCountdown(Math.max(0, Math.ceil(15 - elapsed_)));
    };
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, [dataUpdatedAt]);

  // Keep autoMining ref in sync with state and persist to localStorage
  useEffect(() => {
    autoMiningRef.current = autoMining;
    try {
      localStorage.setItem(MINING_ACTIVE_KEY, autoMining ? '1' : '');
      if (autoMining && miningAddressRef.current) {
        localStorage.setItem(MINING_ADDRESS_KEY, miningAddressRef.current);
      } else if (!autoMining) {
        localStorage.removeItem(MINING_ADDRESS_KEY);
      }
    } catch {
      // ignore
    }
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

    const { blockHash, dataHash, timestamp: blockTimestamp } = latestBlock;
    blockHashRef.current = blockHash;
    dataHashRef.current = dataHash;
    epochIdRef.current = epochId ?? 0;
    blockTimestampRef.current = blockTimestamp;

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
          epochId: epochId ?? 0,
          blockTimestamp: blockTimestamp,
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

      // Safety: don't submit if block data refs aren't populated
      if (!blockHashRef.current || !blockTimestampRef.current) {
        console.warn('[Mining] Skipping submit: block refs not populated yet',
          { blockHash: blockHashRef.current, timestamp: blockTimestampRef.current });
        return;
      }

      const [account] = await signer.getAccounts();
      if (account.address !== address) {
        console.warn('[Mining] Signer address mismatch:', account.address, '!==', address);
        return;
      }
      const msg: SubmitLithiumProofMsg = {
        submit_lithium_proof: {
          hash: proof.hash,
          nonce: Number(proof.nonce),
          miner_address: address,
          block_hash: blockHashRef.current,
          cyberlinks_merkle: dataHashRef.current,
          epoch_id: epochIdRef.current,
          timestamp: blockTimestampRef.current,
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
          LITIUM_MINE_CONTRACT,
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
            referrer,
            blockTimestampRef.current
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
        throw new Error(
          `Submit ${kind} error: ${normalizeErrorText(executeErr)}`
        );
      }

      // Collect all events from both result.events and result.logs[].events[]
      const allEvents: { type: string; attributes?: { key: string; value: string }[] }[] = [];
      if (result.events?.length) {
        allEvents.push(...result.events);
      }
      if ((result as any).logs) {
        for (const log of (result as any).logs) {
          if (log.events?.length) {
            allEvents.push(...log.events);
          }
        }
      }

      console.log(
        '[Mining] Proof submitted! TX:',
        result.transactionHash,
        'events:',
        allEvents.length
      );

      // Extract actual miner reward from wasm event
      let actualReward = 0;
      const wasmEvent = allEvents.find(
        (e) => e.type === 'wasm' && e.attributes?.some(
          (a) => a.key === 'miner_reward' || a.key === 'reward'
        )
      );
      if (wasmEvent) {
        const rewardAttr = wasmEvent.attributes?.find(
          (a) => a.key === 'miner_reward' || a.key === 'reward'
        );
        if (rewardAttr?.value) {
          actualReward = Number(rewardAttr.value) / 1_000_000;
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
          // Compute 30s rolling hashrate on JS side (Rust reports lifetime avg)
          const now = Date.now();
          hashSnapshotsRef.current.push({ time: now, hashes: status.total_hashes });
          const cutoff = now - 30_000;
          hashSnapshotsRef.current = hashSnapshotsRef.current.filter(
            (s) => s.time >= cutoff
          );
          if (hashSnapshotsRef.current.length >= 2) {
            const oldest = hashSnapshotsRef.current[0];
            const newest =
              hashSnapshotsRef.current[hashSnapshotsRef.current.length - 1];
            const dt = (newest.time - oldest.time) / 1000;
            if (dt > 0.5) {
              status = {
                ...status,
                hashrate: (newest.hashes - oldest.hashes) / dt,
              };
            }
          }
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

  // On mount: resume mining if it was active before reload
  useEffect(() => {
    if (isNative) {
      // Tauri: check backend mining state and restore refs from stored params
      let cancelled = false;
      (async () => {
        try {
          const status = (await invoke('get_mining_status')) as MiningStatus & {
            block_hash_hex?: string;
            cyberlinks_merkle_hex?: string;
            epoch_id?: number;
            block_timestamp?: number;
          };
          if (cancelled) return;
          if (status.mining) {
            console.log('[Mining] Detected active mining on mount, resuming UI');
            // Restore refs from Rust-stored params
            if (status.block_hash_hex) {
              blockHashRef.current = status.block_hash_hex;
            }
            if (status.cyberlinks_merkle_hex) {
              dataHashRef.current = status.cyberlinks_merkle_hex;
            }
            if (status.epoch_id !== undefined) {
              epochIdRef.current = status.epoch_id;
            }
            if (status.block_timestamp !== undefined) {
              blockTimestampRef.current = status.block_timestamp;
            }
            console.log('[Mining] Restored refs:', {
              blockHash: blockHashRef.current.slice(0, 16),
              epochId: epochIdRef.current,
              blockTimestamp: blockTimestampRef.current,
            });
            setMiningStatus(status);
            setAutoMining(true);
            startPolling();
          }
        } catch {
          // not in Tauri or mining not initialized
        }
      })();
      return () => { cancelled = true; };
    }

    // WASM: check localStorage flag and auto-restart
    try {
      if (localStorage.getItem(MINING_ACTIVE_KEY)) {
        const savedAddr = localStorage.getItem(MINING_ADDRESS_KEY);
        if (savedAddr && address && savedAddr !== address) {
          console.warn('[Mining] Saved mining address does not match current, skipping resume');
          localStorage.removeItem(MINING_ACTIVE_KEY);
          localStorage.removeItem(MINING_ADDRESS_KEY);
        } else {
          console.log('[Mining] Resuming WASM mining after reload');
          miningAddressRef.current = savedAddr || address;
          setAutoMining(true);
        }
      }
    } catch {
      // ignore
    }
    return undefined;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Auto-start WASM mining when autoMining is true but workers aren't running yet
  // (happens after reload when deps become ready)
  useEffect(() => {
    if (!autoMining || isNative || wasmMinerRef.current || !canMine) return;
    console.log('[Mining] Dependencies ready, starting WASM miners');
    startMiningRound().then(() => startPolling());
  }, [autoMining, canMine, startMiningRound, startPolling, isNative]);

  const handleStartMining = useCallback(async () => {
    miningAddressRef.current = address;
    setAutoMining(true);
    setSessionLiMined(0);
    hashSnapshotsRef.current = [];
    await startMiningRound();
    startPolling();
  }, [startMiningRound, startPolling, address]);

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

  // Stop mining when account switches away from the address that started it
  useEffect(() => {
    if (!autoMining) return;
    if (miningAddressRef.current && address !== miningAddressRef.current) {
      console.warn('[Mining] Account switched, stopping mining');
      handleStopMining();
    }
  }, [address, autoMining, handleStopMining]);

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
              value={compactLi(sessionLiMined)}
              suffix="LI"
            />
            <StatCard
              label="Proofs"
              value={proofLog.filter((p) => p.txHash).length}
            />
            <StatCard
              label="Est. LI/hr"
              value={`~${compactLi(estimatedLiPerHour)}`}
            />
            <StatCard label="Elapsed" value={formatElapsed(elapsed)} />
          </div>

          {/* LI Balance row */}
          <div className={styles.balanceRow}>
            <span>LI Balance</span>
            <span>{compactLi(liBalance)} LI</span>
          </div>

          {/* Emission info */}
          {emission && (
            <div className={styles.sectionBox}>
              <span className={styles.sectionTitle}>Emission (per epoch)</span>
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
              Difficulty: {difficulty ?? '...'}{difficulty !== undefined && ' bits'}
              {minProfitable > 0 && difficulty !== undefined && difficulty < minProfitable && (
                <span style={{ color: '#f5a623' }}> (min profitable: {minProfitable})</span>
              )}
              {' '}· Epoch: {epochId ?? '...'} ·
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
              {' '}· <span style={{ opacity: 0.6 }}>refresh {refreshCountdown}s</span>
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

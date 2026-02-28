import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Display, DisplayTitle, MainContainer } from 'src/components';
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
import StakingSection from './components/StakingSection';
import ReferralSection, { loadReferrer, saveReferrer } from './components/ReferralSection';
import SimulatorSection from './components/SimulatorSection';
import MiningActionBar from './MiningActionBar';
import { useDispatch } from 'react-redux';
import { setMiningActive } from 'src/redux/features/mining';
import { WasmMiner } from './wasmMiner';
import styles from './Mining.module.scss';

type MiningStatus = {
  mining: boolean;
  hashrate: number;
  total_hashes: number;
  elapsed_secs: number;
  pending_proofs: number;
  backend?: string;
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
  const dispatch = useDispatch();
  const { signer, signingClient, address } = useAutoSigner();

  const { data: epochData, refetch: refetchEpoch } = useQueryContract(LITIUM_MINE_CONTRACT, {
    epoch_status: {},
  });
  const { data: difficultyData, refetch: refetchDifficulty } = useQueryContract(LITIUM_MINE_CONTRACT, {
    difficulty: {},
  });
  const { data: targetData } = useQueryContract(LITIUM_MINE_CONTRACT, {
    target: {},
  });
  const { data: proofStatsData, refetch: refetchProofStats } = useQueryContract(LITIUM_MINE_CONTRACT, {
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

  const { block: latestBlock, refetchBlock } = useLatestBlock();
  const { emission } = useEmissionInfo();
  const { burnStats } = useBurnStats();

  const [miningStatus, setMiningStatus] = useState<MiningStatus | null>(null);
  const [autoMining, setAutoMining] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [proofLog, setProofLog] = useState<ProofLogEntry_[]>(loadProofLog);
  const [threadCount, setThreadCount] = useState(() =>
    Math.max(1, (navigator.hardwareConcurrency || 4) - 1)
  );
  const [backend, setBackend] = useState<string>('cpu');
  const [availableBackends, setAvailableBackends] = useState<string[]>(['cpu']);
  const [sessionLiMined, setSessionLiMined] = useState(loadSessionLi);
  const [simOpen, setSimOpen] = useState(false);
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

  // Fetch available backends on mount (Tauri only)
  useEffect(() => {
    if (!isNative) return;
    invoke('get_mining_params')
      .then((params: any) => {
        if (params?.available_backends) {
          setAvailableBackends(params.available_backends);
        }
      })
      .catch(() => {});
  }, [isNative]);

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
  const { networkHashrate, similarDevices, minProfitable } = usePeerEstimate(hashrate);

  // Single 30s interval to refetch contract data — avoids react-query's
  // internal setInterval which causes timer cascade in WebKit
  const [refreshCountdown, setRefreshCountdown] = useState(30);
  const lastRefetchRef = useRef(Date.now());

  useEffect(() => {
    const REFETCH_INTERVAL = 30_000;
    const TICK = 1000;
    const timer = setInterval(() => {
      const elapsed_ = Date.now() - lastRefetchRef.current;
      const remaining = Math.max(0, Math.ceil((REFETCH_INTERVAL - elapsed_) / 1000));
      setRefreshCountdown(remaining);
      if (elapsed_ >= REFETCH_INTERVAL) {
        lastRefetchRef.current = Date.now();
        refetchEpoch();
        refetchDifficulty();
        refetchProofStats();
        refetchBlock();
      }
    }, TICK);
    return () => clearInterval(timer);
  }, [refetchEpoch, refetchDifficulty, refetchProofStats, refetchBlock]);

  // Keep autoMining ref in sync with state, persist to localStorage, and update Redux
  useEffect(() => {
    autoMiningRef.current = autoMining;
    dispatch(setMiningActive(autoMining));
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
  }, [autoMining, dispatch]);

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
          backend,
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
  }, [difficulty, address, latestBlock, epochId, threadCount, backend, isNative]);

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

  // Store processQueue in a ref so the poll interval never needs to be recreated
  const processQueueRef = useRef(processQueue);
  processQueueRef.current = processQueue;

  // Poll mining status and enqueue proofs
  const startPolling = useCallback(() => {
    stopPolling();
    pollRef.current = setInterval(async () => {  // eslint-disable-line
      try {
        let status: MiningStatus;
        if (isNative) {
          status = (await invoke('get_mining_status')) as MiningStatus;
          // Compute 30s rolling hashrate on JS side (Rust reports lifetime avg)
          const now = Date.now();
          const cutoff = now - 30_000;
          while (hashSnapshotsRef.current.length > 0 && hashSnapshotsRef.current[0].time < cutoff) {
            hashSnapshotsRef.current.shift();
          }
          hashSnapshotsRef.current.push({ time: now, hashes: status.total_hashes });
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
            const MAX_QUEUE = 50;
            if (proofQueueRef.current.length < MAX_QUEUE) {
              proofQueueRef.current.push(...proofs.slice(0, MAX_QUEUE - proofQueueRef.current.length));
            }
            console.log(
              `[Mining] ${proofs.length} proof(s) queued, total pending: ${proofQueueRef.current.length}`
            );
          }
        }

        processQueueRef.current();
      } catch (err) {
        console.error('[Mining] Poll error', err);
      }
    }, 1000);
  }, [stopPolling, isNative]);

  // Cleanup on unmount — stop mining in ALL backends to release CPU/GPU
  useEffect(() => {
    return () => {
      stopPolling();
      if (isNative) {
        // Stop Tauri mining thread + release GPU solver
        invoke('stop_mining').catch(() => {});
      }
      if (wasmMinerRef.current) {
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
          {/* Header: wallet + status + simulator toggle */}
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
            <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
              <button
                type="button"
                className={styles.simToggleBtn}
                onClick={() => setSimOpen((v) => !v)}
              >
                {simOpen ? 'Hide Simulator' : 'Simulator'}
              </button>
              <Pill
                color={autoMining ? 'green' : 'black'}
                text={autoMining ? 'Mining' : 'Idle'}
              />
            </div>
          </div>

          {/* Simulator (collapsible) */}
          <SimulatorSection open={simOpen} />

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

          {/* Network info */}
          <div className={styles.sectionBox}>
            <div className={styles.networkHeader}>
              <span className={styles.sectionTitle}>Network</span>
              <span className={styles.refreshBadge}>refresh {refreshCountdown}s</span>
            </div>
            <div className={styles.statsGrid}>
              <StatCard
                label="Difficulty"
                value={difficulty !== undefined ? `${difficulty}` : '...'}
                suffix={difficulty !== undefined ? (
                  minProfitable > 0 && difficulty < minProfitable
                    ? `bits (min: ${minProfitable})`
                    : 'bits'
                ) : undefined}
              />
              <StatCard
                label="Epoch"
                value={epochId ?? '...'}
              />
              <StatCard
                label="Proofs"
                value={`${proofStats?.proof_count ?? '...'} / ${targetSolutions ?? '...'}`}
              />
              <StatCard
                label="My proofs"
                value={minerEpochProofCount}
              />
              <StatCard
                label="Net hashrate"
                value={formatHashrate(networkHashrate)}
              />
              <StatCard
                label="Similar devices"
                value={similarDevices > 0 ? `~${similarDevices}` : '\u2014'}
              />
              <StatCard
                label="All-time miners"
                value={uniqueMiners}
              />
              {latestBlock && (
                <StatCard
                  label="Block"
                  value={latestBlock.height}
                />
              )}
            </div>
          </div>

          {/* Staking section */}
          <StakingSection />

          {/* Referral section */}
          <ReferralSection
            referrer={referrer}
            onReferrerChange={setReferrer}
          />

          {/* Proof summary */}
          {proofLog.length > 0 && (() => {
            const accepted = proofLog.filter((p) => p.txHash).length;
            const failed = proofLog.filter((p) => p.error).length;
            const total = accepted + failed;
            const rate = total > 0 ? ((accepted / total) * 100).toFixed(0) : '—';
            const last = proofLog[0];
            return (
              <div className={styles.sectionBox}>
                <span className={styles.sectionTitle}>Proofs</span>
                <div className={styles.statsGrid}>
                  <StatCard label="Accepted" value={accepted} />
                  <StatCard label="Failed" value={failed} />
                  <StatCard label="Success rate" value={`${rate}%`} />
                </div>
                {last && (
                  <ProofLogEntry
                    index={proofLog.length}
                    hash={last.hash}
                    txHash={last.txHash}
                    error={last.error}
                    timestamp={last.timestamp}
                  />
                )}
              </div>
            );
          })()}
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
        backend={backend}
        onBackendChange={setBackend}
        availableBackends={availableBackends}
        activeBackend={miningStatus?.backend}
        threadCount={threadCount}
        onThreadCountChange={setThreadCount}
        maxThreads={navigator.hardwareConcurrency || 4}
        isNative={isNative}
      />
    </MainContainer>
  );
}

export default Mining;

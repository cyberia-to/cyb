import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Display, DisplayTitle, MainContainer } from 'src/components';
import Pill from 'src/components/Pill/Pill';
import useQueryContract from 'src/hooks/contract/useQueryContract';
import { LITIUM_MINE_CONTRACT, LITIUM_CORE_CONTRACT, UHASH_RELAY_URL, SUBMIT_COOLDOWN_MS } from 'src/constants/mining';
import { RPC_URL } from 'src/constants/config';
import { isTauri } from 'src/utils/tauri';
import { trimString } from 'src/utils/utils';
import { compactLi, formatLi } from './utils/formatLi';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';
import type {
  ExecuteMsg,
  WindowStatusResponse,
  ConfigResponse,
} from 'src/generated/lithium/LitiumMine.types';

type ActivateAccountResponse = {
  ok?: boolean;
  tx_hash?: string;
  error?: string;
};

type SubmitErrorKind =
  | 'account_not_found'
  | 'transport'
  | 'contract'
  | 'unknown';
import useAutoSigner from './hooks/useAutoSigner';
import useRewardEstimate from './hooks/useRewardEstimate';
import useHashrateSamples from './hooks/useHashrateSamples';
import useMinerStats from './hooks/useMinerStats';
import usePeerEstimate from './hooks/usePeerEstimate';
import useLatestBlock from './hooks/useLatestBlock';
import useEmissionInfo from './hooks/useEmissionInfo';
import useBurnStats from './hooks/useBurnStats';
import useNewBlockSubscription from './hooks/useNewBlockSubscription';
import HashrateHero from './components/HashrateHero';
import StatCard from './components/StatCard';
import ProofLogEntry from './components/ProofLogEntry';
import StakingSection from './components/StakingSection';
import ReferralSection, { loadReferrer, saveReferrer } from './components/ReferralSection';
import ConfigPanel from './components/ConfigPanel';
import MiningActionBar from './MiningActionBar';
import DownloadSection from './components/DownloadSection';
import { useAppSelector } from 'src/redux/hooks';
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

type ProofStatus = 'submitted' | 'pending' | 'success' | 'failed';

type ProofLogEntry_ = {
  hash: string;
  nonce: number;
  txHash?: string;
  error?: string;
  status?: ProofStatus;
  timestamp: number;
};

const PROOF_LOG_KEY = 'mining_proof_log';
const SESSION_LI_KEY = 'mining_session_li';
const MINING_ACTIVE_KEY = 'mining_active';
const MINING_ADDRESS_KEY = 'mining_active_address';
const USER_DIFFICULTY_KEY = 'mining_user_difficulty';
const DEFAULT_DIFFICULTY = 12;

function loadProofLog(): ProofLogEntry_[] {
  try {
    const raw = localStorage.getItem(PROOF_LOG_KEY);
    if (!raw) return [];
    const entries: ProofLogEntry_[] = JSON.parse(raw);
    // Migrate legacy entries without status field
    return entries.map((e) => {
      if (!e.status) {
        if (e.error) return { ...e, status: 'failed' as const };
        if (e.txHash) return { ...e, status: 'success' as const };
      }
      return e;
    });
  } catch {
    return [];
  }
}

function saveProofLog(log: ProofLogEntry_[]) {
  try {
    localStorage.setItem(PROOF_LOG_KEY, JSON.stringify(log.slice(0, 200)));
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

function loadUserDifficulty(): number {
  try {
    const saved = localStorage.getItem(USER_DIFFICULTY_KEY);
    if (saved) {
      const n = Number(saved);
      if (n >= 1 && n <= 64) return n;
    }
  } catch {
    // ignore
  }
  return DEFAULT_DIFFICULTY;
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

async function activateAccount(
  minerAddress: string
): Promise<boolean> {
  try {
    const res = await fetch(UHASH_RELAY_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ miner_address: minerAddress }),
    });
    const data = (await res.json()) as ActivateAccountResponse;
    console.log('[Mining] Account activation result:', data);
    return !!data.ok;
  } catch (err) {
    console.error('[Mining] Account activation failed:', err);
    return false;
  }
}

// Module-level WASM miner — survives component unmount/remount during navigation.
// Mining is a background process; only explicit user action (Stop button) or
// closing the app/tab should terminate it.
let persistentWasmMiner: WasmMiner | null = null;

function Mining() {
  const reduxMiningActive = useAppSelector((s) => s.mining.active);
  const defaultAccount = useAppSelector((s) => s.pocket.defaultAccount);
  const { signer, signingClient, address } = useAutoSigner();

  // Window status replaces epoch_status + difficulty + target + proof_stats
  const { data: windowData, refetch: refetchWindow } = useQueryContract(LITIUM_MINE_CONTRACT, {
    window_status: {},
  });
  const { data: configData, refetch: refetchConfig } = useQueryContract(LITIUM_MINE_CONTRACT, {
    config: {},
  });

  const windowStatus = windowData as WindowStatusResponse | undefined;
  const config = configData as ConfigResponse | undefined;
  const minDifficulty = config?.min_difficulty ?? 8;

  // Client-chosen difficulty — persisted to localStorage
  const [userDifficulty, setUserDifficulty] = useState(loadUserDifficulty);
  useEffect(() => {
    try {
      localStorage.setItem(USER_DIFFICULTY_KEY, String(userDifficulty));
    } catch {
      // ignore
    }
  }, [userDifficulty]);

  const { block: latestBlock, refetchBlock } = useLatestBlock();
  const { emission, refetch: refetchEmission } = useEmissionInfo();
  const { burnStats, refetch: refetchBurnStats } = useBurnStats();

  // Mining status from Redux (kept in sync by useMiningMonitor in App.tsx)
  const miningStatus = useAppSelector((s) => s.mining.status);

  const [autoMining, setAutoMining] = useState(() => {
    // Tauri: use Redux state (kept in sync by useMiningMonitor) to avoid flash
    if (isTauri()) return reduxMiningActive;
    try {
      return !!localStorage.getItem(MINING_ACTIVE_KEY);
    } catch {
      return false;
    }
  });
  const [submitting, setSubmitting] = useState(false);
  const [proofLog, setProofLog] = useState<ProofLogEntry_[]>(loadProofLog);
  const [threadCount, setThreadCount] = useState(() =>
    Math.max(1, (navigator.hardwareConcurrency || 4) - 1)
  );
  const [backend, setBackend] = useState<string>('cpu');
  const [availableBackends, setAvailableBackends] = useState<string[]>(['cpu']);
  const [sessionLiMined, setSessionLiMined] = useState(loadSessionLi);
  const [configOpen, setConfigOpen] = useState(false);
  const [proofPage, setProofPage] = useState(1);
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
  const stopReadyRef = useRef(false);
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

  // Track current challenge so proof submission uses the values from when mining started
  const challengeRef = useRef<string>('');
  const blockTimestampRef = useRef<number>(0);

  // Proof submission queue
  const proofQueueRef = useRef<Proof[]>([]);
  const submittingRef = useRef(false);
  const lastSubmitTimeRef = useRef(0);

  const hashrate = miningStatus?.hashrate ?? 0;
  const elapsed = miningStatus?.elapsed_secs ?? 0;
  const canMine = userDifficulty >= minDifficulty && !!address && !!latestBlock;

  const { data: cw20BalData, refetch: refreshBalance } = useQueryContract(
    LITIUM_CORE_CONTRACT,
    address ? { balance: { address } } : { token_info: {} }
  );
  const liBalance = address && cw20BalData && 'balance' in (cw20BalData as object)
    ? Number((cw20BalData as { balance: string }).balance) / 1_000_000
    : 0;
  const { rewardPerProof, grossRewardPerProof, estimatedLiPerHour } =
    useRewardEstimate(userDifficulty, hashrate, emission);
  const samples = useHashrateSamples(hashrate, autoMining);
  const { uniqueMiners, totalProofs, avgDifficulty, refetch: refetchMinerStats } = useMinerStats();
  const {
    dRate, similarDevices, proofCount: windowProofCount, baseRate,
    refetchWindow: refetchPeerWindow,
  } = usePeerEstimate(hashrate);

  // Tiered polling — avoids react-query's internal setInterval which
  // causes timer cascade in WebKit.  Three tiers:
  //   Fast  (10s): window_status, config, block, peer hashrate
  //   Slow (120s): emission, burn stats, all-time miner stats
  // WebSocket NewBlock events trigger fast refetches immediately when
  // available, so the 10s timer acts as a fallback.
  const FAST_INTERVAL = 10_000;
  const SLOW_INTERVAL = 120_000;
  const [refreshCountdown, setRefreshCountdown] = useState(10);
  const lastFastRef = useRef(Date.now());
  const lastSlowRef = useRef(Date.now());

  const refetchFast = useCallback(() => {
    lastFastRef.current = Date.now();
    refetchWindow();
    refetchConfig();
    refetchBlock();
    refetchPeerWindow();
  }, [refetchWindow, refetchConfig, refetchBlock, refetchPeerWindow]);

  const refetchSlow = useCallback(() => {
    lastSlowRef.current = Date.now();
    refetchEmission();
    refetchBurnStats();
    refetchMinerStats();
  }, [refetchEmission, refetchBurnStats, refetchMinerStats]);

  // WebSocket-driven: refetch fast queries on every new block
  const { connected: wsConnected } = useNewBlockSubscription(refetchFast);

  useEffect(() => {
    const TICK = 1000;
    const timer = setInterval(() => {
      const now = Date.now();
      const fastElapsed = now - lastFastRef.current;
      const slowElapsed = now - lastSlowRef.current;

      // Countdown shows time until next fast refetch
      const remaining = Math.max(0, Math.ceil((FAST_INTERVAL - fastElapsed) / 1000));
      setRefreshCountdown(remaining);

      // Fast tier: only poll if WebSocket is not connected (fallback)
      if (!wsConnected && fastElapsed >= FAST_INTERVAL) {
        refetchFast();
      }

      // Slow tier: always poll on timer
      if (slowElapsed >= SLOW_INTERVAL) {
        refetchSlow();
      }
    }, TICK);
    return () => clearInterval(timer);
  }, [wsConnected, refetchFast, refetchSlow]);

  // Keep autoMining ref in sync with state and persist to localStorage.
  // Redux mining state is managed by useMiningMonitor at app level.
  useEffect(() => {
    console.log('[Mining][sync] autoMining changed to:', autoMining);
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
    if (!address || !latestBlock || userDifficulty < minDifficulty) {
      console.log('[Mining] Cannot start: missing address/block or difficulty too low');
      return;
    }

    // Use block hash as the mining challenge (32 bytes hex)
    const challenge = latestBlock.blockHash;
    challengeRef.current = challenge;
    blockTimestampRef.current = latestBlock.timestamp;

    try {
      console.log(
        '[Mining] Starting lithium mining, difficulty:',
        userDifficulty,
        'challenge:',
        challenge.slice(0, 16)
      );

      if (isNative) {
        await invoke('start_mining', {
          address,
          challengeHex: challenge,
          difficulty: userDifficulty,
          blockTimestamp: latestBlock.timestamp,
          threads: threadCount,
          backend,
        });
      } else {
        if (!persistentWasmMiner) {
          const miner = new WasmMiner(threadCount);
          await miner.init();
          persistentWasmMiner = miner;
        }
        wasmMinerRef.current = persistentWasmMiner;
        persistentWasmMiner.start(challenge, userDifficulty);
      }
    } catch (err) {
      console.error('[Mining] Failed to start mining', err);
    }
  }, [userDifficulty, minDifficulty, address, latestBlock, threadCount, backend, isNative]);

  // Submit a single proof to chain
  const submitSingleProof = useCallback(
    async (proof: Proof) => {
      if (!signer || !signingClient || !address) {
        console.log('[Mining] Cannot submit: no signer/client');
        return;
      }

      // Safety: don't submit if challenge refs aren't populated
      if (!challengeRef.current || !blockTimestampRef.current) {
        console.warn('[Mining] Skipping submit: challenge refs not populated yet',
          { challenge: challengeRef.current, timestamp: blockTimestampRef.current });
        return;
      }

      const [account] = await signer.getAccounts();
      if (account.address !== address) {
        console.warn('[Mining] Signer address mismatch:', account.address, '!==', address);
        return;
      }
      const msg: ExecuteMsg = {
        submit_proof: {
          hash: proof.hash,
          nonce: Number(proof.nonce),
          miner_address: address,
          challenge: challengeRef.current,
          difficulty: userDifficulty,
          timestamp: blockTimestampRef.current,
          referrer: referrer || undefined,
        },
      };

      console.log(
        '[Mining] Submitting proof:',
        `${proof.hash.slice(0, 16)}...`,
        'difficulty:',
        userDifficulty
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
          console.log('[Mining] Account not on chain, activating...');
          const activated = await activateAccount(account.address);
          if (activated) {
            console.log('[Mining] Account activated, waiting for tx inclusion...');
            // Wait for activation tx to be included in a block
            await new Promise<void>((resolve) => {
              setTimeout(resolve, 7000);
            });
            // Retry proof submission directly
            console.log('[Mining] Retrying proof submission...');
            try {
              result = await signingClient.execute(
                account.address,
                LITIUM_MINE_CONTRACT,
                msg,
                Soft3MessageFactory.fee(8),
                ''
              );
            } catch (retryErr) {
              if (isNative) {
                invoke('report_proof_failed').catch(() => {});
              }
              throw new Error(
                `Retry after activation failed: ${normalizeErrorText(retryErr)}`
              );
            }
          } else {
            if (isNative) {
              invoke('report_proof_failed').catch(() => {});
            }
            throw new Error('Account activation failed — cannot submit proof');
          }
        } else {
          if (isNative) {
            invoke('report_proof_failed').catch(() => {});
          }
          throw new Error(
            `Submit ${kind} error: ${normalizeErrorText(executeErr)}`
          );
        }
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

      // Report successful submission to Tauri backend for metrics
      if (isNative) {
        invoke('report_proof_submitted').catch(() => {});
      }

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

      // Update submitted entry to success with txHash
      setProofLog((prev) =>
        prev.map((p) =>
          p.hash === proof.hash && (p.status === 'submitted' || !p.txHash)
            ? { ...p, txHash: result.transactionHash, status: 'success' }
            : p
        )
      );
      refreshBalance();
      setSessionLiMined((prev) => prev + actualReward);
    },
    [signer, signingClient, address, userDifficulty, referrer, refreshBalance, rewardPerProof, isNative]
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

    // Add "submitted" entry immediately
    setProofLog((prev) => [
      {
        hash: best.hash,
        nonce: best.nonce,
        status: 'submitted',
        timestamp: Date.now(),
      },
      ...prev,
    ]);

    try {
      await submitSingleProof(best);
      lastSubmitTimeRef.current = Date.now();
    } catch (err: any) {
      console.error('[Mining] Submit failed:', err);
      // Update the submitted entry to failed
      setProofLog((prev) =>
        prev.map((p) =>
          p.hash === best.hash && p.status === 'submitted'
            ? { ...p, error: err?.message || 'Failed', status: 'failed' }
            : p
        )
      );
    } finally {
      submittingRef.current = false;
      setSubmitting(false);
    }
  }, [submitSingleProof]);

  // Store processQueue in a ref so the poll interval never needs to be recreated
  const processQueueRef = useRef(processQueue);
  processQueueRef.current = processQueue;

  // Poll for proof submission only.
  // Mining status display is handled by useMiningMonitor → Redux.
  const startPolling = useCallback(() => {
    stopPolling();
    pollRef.current = setInterval(async () => {  // eslint-disable-line
      try {
        if (!autoMiningRef.current) return;

        // Check pending proofs from backend
        let pendingProofs = 0;
        if (isNative) {
          const status = (await invoke('get_mining_status')) as MiningStatus;
          pendingProofs = status.pending_proofs;
        } else if (wasmMinerRef.current) {
          pendingProofs = wasmMinerRef.current.getStatus().pending_proofs;
        }

        if (pendingProofs > 0) {
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

  // Cleanup on unmount — stop polling only.
  // Mining is a background process that persists across navigation:
  //   Tauri: Rust backend keeps running
  //   WASM: module-level persistentWasmMiner keeps workers alive
  // Only explicit Stop button or closing the app/tab kills mining.
  useEffect(() => {
    // Block stale click events from previous page's ActionBar hitting
    // our Stop button in the same event loop tick as mount.
    stopReadyRef.current = false;
    const raf = requestAnimationFrame(() => { stopReadyRef.current = true; });
    console.log('[Mining][lifecycle] mount, isNative:', isNative);
    return () => {
      cancelAnimationFrame(raf);
      console.log('[Mining][lifecycle] unmount cleanup (polling only), autoMining:', autoMiningRef.current);
      stopPolling();
    };
  }, [stopPolling]);

  // On mount: resume mining if it was active before reload
  useEffect(() => {
    if (isNative) {
      // Tauri: check backend mining state and restore refs from stored params
      let cancelled = false;
      console.log('[Mining][resume] checking backend state...');
      (async () => {
        try {
          const status = (await invoke('get_mining_status')) as MiningStatus & {
            challenge_hex?: string;
            block_timestamp?: number;
          };
          console.log('[Mining][resume] got status, mining:', status.mining, 'cancelled:', cancelled);
          if (cancelled) return;
          if (status.mining) {
            console.log('[Mining][resume] resuming UI, address:', address?.slice(0, 16));
            // Restore refs from Rust-stored params
            if (status.challenge_hex) {
              challengeRef.current = status.challenge_hex;
            }
            if (status.block_timestamp !== undefined) {
              blockTimestampRef.current = status.block_timestamp;
            }
            miningAddressRef.current = address;
            setAutoMining(true);
            startPolling();
          } else {
            console.log('[Mining][resume] backend not mining');
          }
        } catch (err) {
          console.log('[Mining][resume] error:', err);
        }
      })();
      return () => {
        console.log('[Mining][resume] cleanup, setting cancelled=true');
        cancelled = true;
      };
    }

    // WASM: check if persistent miner is still running (survives navigation)
    if (persistentWasmMiner) {
      const status = persistentWasmMiner.getStatus();
      if (status.mining) {
        console.log('[Mining][resume] persistent WASM miner still running, resuming UI');
        wasmMinerRef.current = persistentWasmMiner;
        miningAddressRef.current = localStorage.getItem(MINING_ADDRESS_KEY) || address;
        setAutoMining(true);
        startPolling();
        return undefined;
      }
    }

    // WASM: autoMining is already initialized from localStorage in useState.
    // Restore miningAddressRef so the auto-start effect can proceed.
    if (autoMining) {
      try {
        const savedAddr = localStorage.getItem(MINING_ADDRESS_KEY);
        if (savedAddr && address && savedAddr !== address) {
          console.warn('[Mining] Saved mining address does not match current, stopping');
          setAutoMining(false);
        } else {
          miningAddressRef.current = savedAddr || address;
        }
      } catch {
        // ignore
      }
    }
    return undefined;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Auto-start WASM mining when autoMining is true but workers aren't running yet
  // (happens after reload when deps become ready)
  useEffect(() => {
    if (!autoMining || isNative || persistentWasmMiner || !canMine) return;
    console.log('[Mining] Dependencies ready, starting WASM miners');
    startMiningRound().then(() => startPolling());
  }, [autoMining, canMine, startMiningRound, startPolling, isNative]);

  // Hot-swap challenge when a new block arrives while mining is active.
  // Without this, proofs are mined against a stale block hash and the contract
  // rejects them with "epoch mismatch".
  useEffect(() => {
    if (!autoMining || !latestBlock) return;
    const newChallenge = latestBlock.blockHash;
    if (newChallenge === challengeRef.current) return;

    console.log(
      '[Mining] Block changed, updating challenge:',
      newChallenge.slice(0, 16),
      'height:',
      latestBlock.height
    );
    challengeRef.current = newChallenge;
    blockTimestampRef.current = latestBlock.timestamp;

    if (isNative) {
      invoke('update_challenge', {
        challengeHex: newChallenge,
        blockTimestamp: latestBlock.timestamp,
      }).catch((err) => console.warn('[Mining] update_challenge failed:', err));
    } else if (persistentWasmMiner) {
      persistentWasmMiner.start(newChallenge, userDifficulty);
    }
  }, [autoMining, latestBlock, userDifficulty, isNative]);

  // Auto-adjust difficulty if contract min_difficulty increases above user setting
  useEffect(() => {
    if (config && config.min_difficulty > userDifficulty) {
      console.log('[Mining] Config min_difficulty increased, adjusting:', config.min_difficulty);
      setUserDifficulty(config.min_difficulty);
    }
  }, [config, userDifficulty]);

  const handleStartMining = useCallback(async () => {
    miningAddressRef.current = address;
    setAutoMining(true);
    setSessionLiMined(0);
    await startMiningRound();
    startPolling();
  }, [startMiningRound, startPolling, address]);

  const handleStopMining = useCallback(async () => {
    // Guard: ignore click events from the same frame as mount (stale click-through
    // from previous page's ActionBar back button hitting our Stop button).
    if (!stopReadyRef.current) {
      console.log('[Mining][stop] IGNORED — stale click before first frame');
      return;
    }
    console.log('[Mining][stop] handleStopMining called', new Error().stack?.split('\n').slice(1, 4).join(' <- '));
    setAutoMining(false);
    try {
      if (isNative) {
        await invoke('stop_mining');
      } else if (persistentWasmMiner) {
        persistentWasmMiner.destroy();
        persistentWasmMiner = null;
        wasmMinerRef.current = null;
      }
    } catch (err) {
      console.error('[Mining] Failed to stop mining', err);
    }
    stopPolling();
  }, [stopPolling, isNative]);

  // Stop mining when account switches away from the address that started it
  useEffect(() => {
    console.log('[Mining][account-switch] effect, autoMining:', autoMining, 'address:', address?.slice(0, 16), 'miningAddressRef:', miningAddressRef.current?.slice(0, 16));
    if (!autoMining) return;
    if (miningAddressRef.current && address !== miningAddressRef.current) {
      console.warn('[Mining][account-switch] STOPPING: address mismatch', address, '!==', miningAddressRef.current);
      handleStopMining();
    }
  }, [address, autoMining, handleStopMining]);

  // Stop mining if contract is paused
  useEffect(() => {
    console.log('[Mining][pause-check] effect, paused:', config?.paused, 'autoMining:', autoMining);
    if (config?.paused && autoMining) {
      console.log('[Mining][pause-check] STOPPING: contract paused');
      handleStopMining();
    }
  }, [config?.paused, autoMining, handleStopMining]);

  const handleCopyAddress = useCallback(() => {
    if (address) {
      navigator.clipboard.writeText(address);
    }
  }, [address]);

  const handleExportLogs = useCallback(async () => {
    const accepted = proofLog.filter((p) => p.status === 'success').length;
    const failed = proofLog.filter((p) => p.status === 'failed' || (p.error && !p.status)).length;
    const pending = proofLog.filter((p) => p.status === 'submitted' || p.status === 'pending').length;
    const total = accepted + failed;

    // Fetch live Tauri backend metrics (includes batch_count, avg_batch_ms,
    // proofs_submitted, proofs_failed that aren't in Redux)
    let tauriStatus: Record<string, unknown> | null = null;
    let tauriParams: Record<string, unknown> | null = null;
    if (isNative) {
      try {
        const [status, params] = await Promise.all([
          invoke('get_mining_status') as Promise<Record<string, unknown>>,
          invoke('get_mining_params') as Promise<Record<string, unknown>>,
        ]);
        tauriStatus = status;
        tauriParams = params;
      } catch {
        // backend unavailable
      }
    }

    // Device info available on all platforms
    const nav = navigator as Navigator & { deviceMemory?: number };
    const device = {
      cpu_cores: navigator.hardwareConcurrency || null,
      device_memory_gb: nav.deviceMemory || null,
      platform: navigator.platform || null,
      max_touch_points: navigator.maxTouchPoints,
    };

    const report = {
      exported_at: new Date().toISOString(),
      user_agent: navigator.userAgent,
      platform: isNative ? 'tauri' : 'web',
      address: address || null,
      referrer: referrer || null,
      li_balance: liBalance,
      device,
      mining: {
        active: autoMining,
        difficulty: userDifficulty,
        min_difficulty: minDifficulty,
        threads: threadCount,
        backend,
        available_backends: availableBackends,
        hashrate,
        total_hashes: miningStatus?.total_hashes ?? 0,
        elapsed_secs: elapsed,
        pending_proofs: miningStatus?.pending_proofs ?? 0,
        session_li_mined: sessionLiMined,
        // Tauri-only batch/proof counters
        batch_count: (tauriStatus?.batch_count as number) ?? null,
        avg_batch_ms: (tauriStatus?.avg_batch_ms as number) ?? null,
        proofs_submitted: (tauriStatus?.proofs_submitted as number) ?? null,
        proofs_failed: (tauriStatus?.proofs_failed as number) ?? null,
      },
      // uhash algorithm params (Tauri-only)
      uhash_params: tauriParams
        ? {
            chains: tauriParams.chains,
            scratchpad_kb: tauriParams.scratchpad_kb,
            total_mb: tauriParams.total_mb,
            rounds: tauriParams.rounds,
            block_size: tauriParams.block_size,
          }
        : null,
      block: latestBlock
        ? { height: latestBlock.height, hash: latestBlock.blockHash, timestamp: latestBlock.timestamp }
        : null,
      ws_connected: wsConnected,
      hashrate_samples: samples,
      contract_config: config || null,
      window_status: windowStatus || null,
      emission: emission || null,
      burn_stats: burnStats || null,
      network: {
        unique_miners: uniqueMiners,
        total_proofs: totalProofs,
        avg_difficulty: avgDifficulty,
        d_rate: dRate,
        similar_devices: similarDevices,
        window_proof_count: windowProofCount,
        base_rate: baseRate,
      },
      reward_estimate: {
        per_proof: rewardPerProof,
        gross_per_proof: grossRewardPerProof,
        li_per_hour: estimatedLiPerHour,
      },
      proof_summary: {
        total: proofLog.length,
        accepted,
        failed,
        pending,
        success_rate: total > 0 ? `${((accepted / total) * 100).toFixed(1)}%` : null,
      },
      proof_log: proofLog,
    };

    const text = JSON.stringify(report, null, 2);
    const blob = new Blob([text], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `mining-log-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }, [
    address, referrer, liBalance, autoMining, userDifficulty, minDifficulty,
    threadCount, backend, availableBackends, hashrate, miningStatus, elapsed,
    sessionLiMined, latestBlock, wsConnected, samples, config, windowStatus,
    emission, burnStats, uniqueMiners, totalProofs, avgDifficulty, dRate,
    similarDevices, windowProofCount, baseRate, rewardPerProof,
    grossRewardPerProof, estimatedLiPerHour, proofLog, isNative,
  ]);

  // Format alpha/beta from micros to percentage
  const alphaPercent = config ? (config.alpha / 10_000).toFixed(1) : '...';
  const betaPercent = config ? (config.beta / 10_000).toFixed(1) : '...';

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
                onClick={() => setConfigOpen((v) => !v)}
              >
                {configOpen ? 'Hide Config' : 'Config'}
              </button>
              <button
                type="button"
                className={styles.simToggleBtn}
                onClick={handleExportLogs}
                title="Download mining report as JSON file"
              >
                Export Logs
              </button>
              {!autoMining && <Pill color="black" text="Idle" />}
            </div>
          </div>

          {/* Config panel (collapsible) */}
          <ConfigPanel open={configOpen} config={config} onConfigUpdated={refetchConfig} />

          {/* Desktop download CTA (web only — first thing users see) */}
          {!isNative && <DownloadSection address={address} accountName={defaultAccount?.name || undefined} />}

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
              value={proofLog.filter((p) => p.status === 'success').length}
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
              <span className={styles.sectionTitle}>Emission (per second)</span>
              <div className={styles.statsGrid}>
                <StatCard
                  label="Mining"
                  value={formatLi(emission.mining_rate)}
                  suffix="LI/s"
                />
                <StatCard
                  label="Staking"
                  value={formatLi(emission.staking_rate)}
                  suffix="LI/s"
                />
                <StatCard
                  label="Gross rate"
                  value={formatLi(emission.gross_rate)}
                  suffix="LI/s"
                />
                <StatCard
                  label="Windowed fees"
                  value={formatLi(emission.windowed_fees)}
                  suffix="LI"
                />
                {burnStats && (
                  <StatCard
                    label="Total burned"
                    value={formatLi(burnStats.total_burned)}
                    suffix="LI"
                  />
                )}
              </div>
            </div>
          )}

          {/* Network info */}
          <div className={styles.sectionBox}>
            <div className={styles.networkHeader}>
              <span className={styles.sectionTitle}>Network</span>
              <span className={styles.refreshBadge}>
                {wsConnected ? 'live' : `refresh ${refreshCountdown}s`}
              </span>
            </div>
            <div className={styles.statsGrid}>
              <StatCard
                label="Your difficulty"
                value={`${userDifficulty}`}
                suffix={`bits (min: ${minDifficulty})`}
              />
              <StatCard
                label="Base rate"
                value={baseRate !== '0' ? formatLi(baseRate) : '...'}
                suffix="LI/bit"
              />
              <StatCard
                label="Window proofs"
                value={windowProofCount}
              />
              <StatCard
                label="D-rate"
                value={dRate > 0 ? dRate.toFixed(2) : '...'}
                suffix="bits/s"
              />
              <StatCard
                label="Alpha"
                value={`${alphaPercent}%`}
              />
              <StatCard
                label="Beta"
                value={`${betaPercent}%`}
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

          {/* Proof summary + paginated list */}
          {proofLog.length > 0 && (() => {
            const accepted = proofLog.filter((p) => p.status === 'success').length;
            const failed = proofLog.filter((p) => p.status === 'failed' || (p.error && !p.status)).length;
            const pending = proofLog.filter((p) => p.status === 'submitted' || p.status === 'pending').length;
            const total = accepted + failed;
            const rate = total > 0 ? ((accepted / total) * 100).toFixed(0) : '\u2014';
            const PAGE_SIZE = 20;
            const visibleProofs = proofLog.slice(0, proofPage * PAGE_SIZE);
            const hasMore = proofLog.length > visibleProofs.length;
            return (
              <div className={styles.sectionBox}>
                <span className={styles.sectionTitle}>Proofs</span>
                <div className={styles.statsGrid}>
                  <StatCard label="Accepted" value={accepted} />
                  <StatCard label="Failed" value={failed} />
                  <StatCard label="Success rate" value={`${rate}%`} />
                </div>
                <div className={styles.proofLog}>
                  {visibleProofs.map((p, i) => (
                    <ProofLogEntry
                      key={`${p.hash}-${p.timestamp}`}
                      index={proofLog.length - i}
                      hash={p.hash}
                      txHash={p.txHash}
                      error={p.error}
                      status={p.status}
                      timestamp={p.timestamp}
                    />
                  ))}
                </div>
                {hasMore && (
                  <button
                    type="button"
                    className={styles.showMoreBtn}
                    onClick={() => setProofPage((p) => p + 1)}
                  >
                    Show more ({proofLog.length - visibleProofs.length} remaining)
                  </button>
                )}
              </div>
            );
          })()}
        </div>
      </Display>

      <MiningActionBar
        difficulty={userDifficulty}
        minDifficulty={minDifficulty}
        address={address}
        blockReady={!!latestBlock}
        autoMining={autoMining}
        submitting={submitting}
        miningStatus={miningStatus}
        onStartMining={handleStartMining}
        onStopMining={handleStopMining}
        onDifficultyChange={setUserDifficulty}
        backend={backend}
        onBackendChange={setBackend}
        availableBackends={availableBackends}
        activeBackend={miningStatus?.backend}
        threadCount={threadCount}
        onThreadCountChange={setThreadCount}
        maxThreads={Math.max(1, (navigator.hardwareConcurrency || 4) - 1)}
        isNative={isNative}
      />
    </MainContainer>
  );
}

export default Mining;

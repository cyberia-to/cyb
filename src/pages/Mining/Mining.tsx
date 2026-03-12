import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Display, DisplayTitle, MainContainer } from 'src/components';
import Pill from 'src/components/Pill/Pill';
import useQueryContract from 'src/hooks/contract/useQueryContract';
import { LITIUM_MINE_CONTRACT, LITIUM_CORE_CONTRACT, LITIUM_STAKE_CONTRACT, UHASH_RELAY_URL, SUBMIT_COOLDOWN_MS } from 'src/constants/mining';
import { CHAIN_ID, RPC_URL } from 'src/constants/config';
import { isTauri } from 'src/utils/tauri';
import { trimString } from 'src/utils/utils';
import { compactLi, formatLi } from './utils/formatLi';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';
import { toUtf8 } from '@cosmjs/encoding';
import { TxRaw } from 'cosmjs-types/cosmos/tx/v1beta1/tx';
import type {
  ExecuteMsg,
  WindowStatusResponse,
  ConfigResponse,
} from 'src/generated/lithium/LitiumMine.types';
import type { TotalMintedResponse } from 'src/generated/lithium/LitiumCore.types';
import type { TotalStakedResponse } from 'src/generated/lithium/LitiumStake.types';

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
import useNewBlockSubscription from './hooks/useNewBlockSubscription';
import useCountdown from './hooks/useCountdown'; // TESTING: countdown — remove after testing phase
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
import { buildMiningReport, type ReportParams } from './buildMiningReport';
import { useAutoSaveLogs } from './hooks/useAutoSaveLogs';
import styles from './Mining.module.scss';

type MiningStatus = {
  mining: boolean;
  hashrate: number;
  total_hashes: number;
  elapsed_secs: number;
  pending_proofs: number;
  backend?: string;
};

type ProofStatus = 'submitted' | 'pending' | 'success' | 'failed' | 'retrying';

type RetryEntry = {
  proof: Proof;
  challenge: string;
  blockTimestamp: number;
  attempts: number;
  nextRetryAt: number;
};

const MAX_RETRY_QUEUE = 10;
const MAX_RETRY_ATTEMPTS = 3;
const RETRY_BACKOFF_BASE = 6000; // 6s, 12s, 24s
const CONSECUTIVE_FAILS_THRESHOLD = 3;

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
const COUNTDOWN_MODE_KEY = 'mining_countdown_mode';
const DEFAULT_DIFFICULTY = 12;

function loadProofLog(): ProofLogEntry_[] {
  try {
    const raw = localStorage.getItem(PROOF_LOG_KEY);
    if (!raw) return [];
    const entries: ProofLogEntry_[] = JSON.parse(raw);
    return entries.map((e) => {
      // Migrate legacy entries without status field
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

type Proof = { hash: string; nonce: number; challenge?: string };

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

/** Accessor for app-level polling (useMiningMonitor) */
export function getWasmMiner(): WasmMiner | null {
  return persistentWasmMiner;
}

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
  const { data: totalMintedData, refetch: refetchTotalMinted } = useQueryContract(
    LITIUM_CORE_CONTRACT,
    { total_minted: {} }
  );
  const totalMinted = totalMintedData as TotalMintedResponse | undefined;
  const { data: totalStakedData, refetch: refetchTotalStaked } = useQueryContract(
    LITIUM_STAKE_CONTRACT,
    { total_staked: {} }
  );
  const totalStaked = totalStakedData as TotalStakedResponse | undefined;

  // PoW share from real on-chain state: powShare = 1 - S^alpha
  // S = total_staked / total_minted, alpha from window_status (Decimal string)
  const stakedFraction = totalMinted && totalStaked && Number(totalMinted.total_minted) > 0
    ? Number(totalStaked.total_staked) / Number(totalMinted.total_minted)
    : 0;
  const alphaExp = windowStatus ? Number(windowStatus.alpha) : 0;
  const powShare = stakedFraction > 0 && alphaExp > 0
    ? 1 - Math.pow(stakedFraction, alphaExp)
    : 1;

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
  const [cpuCores, setCpuCores] = useState(() => navigator.hardwareConcurrency || 4);
  const [threadCount, setThreadCount] = useState(() => {
    const cores = navigator.hardwareConcurrency || 4;
    if (isTauri()) {
      return Math.max(1, cores - 1); // Native: use all but 1 core
    }
    return Math.max(1, Math.floor(cores / 2)); // WASM: use half cores to stay responsive
  });
  const [backend, setBackend] = useState<string>('cpu');
  const [availableBackends, setAvailableBackends] = useState<string[]>(['cpu']);
  const [sessionLiMined, setSessionLiMined] = useState(loadSessionLi);
  const [configOpen, setConfigOpen] = useState(false);
  const [countdownMode, setCountdownMode] = useState(() => {
    try { return !!localStorage.getItem(COUNTDOWN_MODE_KEY); } catch { return false; }
  });
  useEffect(() => {
    try {
      if (countdownMode) localStorage.setItem(COUNTDOWN_MODE_KEY, '1');
      else localStorage.removeItem(COUNTDOWN_MODE_KEY);
    } catch { /* ignore */ }
  }, [countdownMode]);
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
  const sessionStart = useAppSelector((s) => s.mining.sessionStart) ?? Date.now();
  const isNative = isTauri();

  // Fetch available backends on mount (Tauri only)
  useEffect(() => {
    if (!isNative) return;
    invoke('get_mining_params')
      .then((params: any) => {
        if (params?.available_backends) {
          setAvailableBackends(params.available_backends);
        }
        if (params?.cpu_cores && params.cpu_cores > 0) {
          const cores = params.cpu_cores as number;
          setCpuCores(cores);
          // Bump thread count if it was capped by navigator.hardwareConcurrency
          setThreadCount((prev) => {
            const browserMax = Math.max(1, (navigator.hardwareConcurrency || 4) - 1);
            return prev >= browserMax ? Math.max(1, cores - 1) : prev;
          });
        }
      })
      .catch(() => {});
  }, [isNative]);

  // Track current challenge so proof submission uses the values from when mining started
  const challengeRef = useRef<string>('');
  const blockTimestampRef = useRef<number>(0);
  const challengeCreatedAtRef = useRef<number>(0);

  // Connection health
  const [rpcOnline, setRpcOnline] = useState(true);
  const consecutiveFailsRef = useRef(0);
  const retryQueueRef = useRef<RetryEntry[]>([]);

  // Local sequence tracking — avoids re-querying chain between proofs
  const seqRef = useRef<{ accountNumber: number; sequence: number } | null>(null);

  // Proof submission queue
  const proofQueueRef = useRef<Proof[]>([]);
  const submittingRef = useRef(false);
  const lastSubmitTimeRef = useRef(0);

  const hashrate = miningStatus?.hashrate ?? 0;
  const elapsed = miningStatus?.elapsed_secs ?? 0;
  const canMine = userDifficulty >= minDifficulty && !!address && !!latestBlock;

  // TESTING: Genesis countdown — remove after testing phase
  const genesisInFuture = config?.genesis_time
    ? config.genesis_time > Math.floor(Date.now() / 1000)
    : false;
  // Always tick when genesis is in the future (show countdown immediately on page load)
  const countdownTarget = genesisInFuture ? config?.genesis_time : undefined;
  const countdownSeconds = useCountdown(countdownTarget);


  const { data: cw20BalData, refetch: refreshBalance } = useQueryContract(
    LITIUM_CORE_CONTRACT,
    address ? { balance: { address } } : { token_info: {} }
  );
  const liBalance = address && cw20BalData && 'balance' in (cw20BalData as object)
    ? Number((cw20BalData as { balance: string }).balance) / 1_000_000
    : 0;
  const { rewardPerProof, grossRewardPerProof, estimatedLiPerHour, refetch: refetchReward } =
    useRewardEstimate(userDifficulty, hashrate, powShare);
  const samples = useHashrateSamples(hashrate, autoMining);
  const { uniqueMiners, totalProofs, avgDifficulty, refetch: refetchMinerStats } = useMinerStats();
  const {
    dRate, similarDevices, windowEntries, windowSize, baseRate,
    refetchWindow: refetchPeerWindow,
  } = usePeerEstimate(hashrate);

  // Event-driven refresh — zero timer polling.
  // Three triggers: new block (WS), after proof submit, platform recovery.
  // WS fallback: 30s interval only when WebSocket is disconnected.
  const refetchAll = useCallback(() => {
    refetchWindow();
    refetchBlock();
    refetchPeerWindow();
    refetchTotalMinted();
    refetchTotalStaked();
    refetchMinerStats();
    refetchReward();
  }, [refetchWindow, refetchBlock, refetchPeerWindow, refetchTotalMinted, refetchTotalStaked, refetchMinerStats, refetchReward]);

  // Resync config + balance after proof submit (success or permanent failure)
  const resyncAfterProof = useCallback(() => {
    refetchConfig();
    refreshBalance();
  }, [refetchConfig, refreshBalance]);

  // WebSocket-driven: refetch all display data on every new block
  const { connected: wsConnected } = useNewBlockSubscription(refetchAll);

  // WS disconnection fallback: 30s interval, only active when WS is down
  useEffect(() => {
    if (wsConnected) return;
    // Immediate refresh when WS drops
    refetchAll();
    refetchConfig();
    const timer = setInterval(() => {
      refetchAll();
      refetchConfig();
    }, 30_000);
    return () => clearInterval(timer);
  }, [wsConnected, refetchAll, refetchConfig]);

  // Handle sleep/wake and network reconnection using platform-native events.
  // Desktop Tauri: window focus event (fires on Cmd+Tab, dock click, sleep/wake)
  // Mobile Tauri:  'app-resumed' custom event (emitted from Rust on RunEvent::Resumed)
  // Web:           visibilitychange (standard browser API)
  // All:           'online' event for network reconnection
  useEffect(() => {
    const cleanups: (() => void)[] = [];

    // Debounce: don't refetch more than once per 5s from any signal
    let lastRefetch = 0;
    const debouncedRefetch = async (source: string) => {
      const now = Date.now();
      if (now - lastRefetch < 5000) return;
      lastRefetch = now;
      console.log(`[Mining][wake] ${source}, refreshing...`);
      refetchAll();
      refetchConfig();

      // Probe RPC health to restore online state
      try {
        const probeOk = await fetch(RPC_URL + '/status', {
          signal: AbortSignal.timeout(5000),
        }).then((r) => r.ok).catch(() => false);
        if (probeOk) {
          setRpcOnline(true);
          consecutiveFailsRef.current = 0;
          console.log('[Mining][wake] RPC probe OK — marking online');
        }
      } catch {
        // probe failed, stay in current state
      }
    };

    if (isNative) {
      let cancelled = false;

      // Desktop: window focus event
      import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
        if (cancelled) return;
        getCurrentWindow().onFocusChanged(({ payload: focused }) => {
          if (focused) {
            debouncedRefetch('Window focused');
          }
        }).then((unlisten) => {
          if (cancelled) { unlisten(); } else { cleanups.push(unlisten); }
        });
      });

      // Mobile: app-resumed event (RunEvent::Resumed — iOS/Android foreground)
      import('@tauri-apps/api/event').then(({ listen }) => {
        if (cancelled) return;
        listen('app-resumed', () => {
          debouncedRefetch('App resumed (mobile)');
        }).then((unlisten) => {
          if (cancelled) { unlisten(); } else { cleanups.push(unlisten); }
        });
      });

      cleanups.push(() => { cancelled = true; });
    } else {
      // Web: use visibilitychange
      const handleVisibility = () => {
        if (!document.hidden) {
          debouncedRefetch('Page visible');
        }
      };
      document.addEventListener('visibilitychange', handleVisibility);
      cleanups.push(() => document.removeEventListener('visibilitychange', handleVisibility));
    }

    // All platforms: network reconnection
    const handleOnline = () => debouncedRefetch('Network online');
    window.addEventListener('online', handleOnline);
    cleanups.push(() => window.removeEventListener('online', handleOnline));

    return () => cleanups.forEach((fn) => fn());
  }, [refetchAll, refetchConfig, isNative]);

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

  // Resolve stale 'submitted'/'retrying' entries from previous session on mount
  useEffect(() => {
    const stale = proofLog.filter(
      (p) => p.status === 'submitted' || p.status === 'retrying'
    );
    if (stale.length === 0) return;

    // Entries without txHash were never broadcast — resolve immediately
    const neverBroadcast = stale.filter((p) => !p.txHash);
    if (neverBroadcast.length > 0) {
      setProofLog((prev) =>
        prev.map((p) =>
          (p.status === 'submitted' || p.status === 'retrying') && !p.txHash
            ? { ...p, status: 'failed' as const, error: 'Never broadcast' }
            : p
        )
      );
    }

    // Entries with txHash — verify on-chain once client is ready
    if (!signingClient) return;
    const withTx = stale.filter((p) => p.txHash);
    if (withTx.length === 0) return;

    (async () => {
      const updates: Record<string, { status: ProofStatus; error?: string }> = {};
      for (const entry of withTx) {
        try {
          const tx = await signingClient.getTx(entry.txHash!);
          if (tx) {
            updates[entry.hash] = tx.code === 0
              ? { status: 'success' }
              : { status: 'failed', error: tx.rawLog || `code ${tx.code}` };
          } else {
            updates[entry.hash] = { status: 'failed', error: 'TX not found on chain' };
          }
        } catch {
          // Can't verify — leave as-is, will retry next launch
        }
      }
      if (Object.keys(updates).length > 0) {
        setProofLog((prev) =>
          prev.map((p) =>
            updates[p.hash] ? { ...p, ...updates[p.hash] } : p
          )
        );
      }
    })();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [!!signingClient]);

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
    if (!address || userDifficulty < minDifficulty) {
      console.log('[Mining] Cannot start: missing address or difficulty too low');
      return;
    }

    // Native path still uses block hash (Tauri backend manages its own challenge rotation)
    if (isNative) {
      if (!latestBlock) {
        console.log('[Mining] Cannot start native: no block data');
        return;
      }
      const challenge = latestBlock.blockHash;
      challengeRef.current = challenge;
      blockTimestampRef.current = latestBlock.timestamp;
      try {
        console.log('[Mining] Starting native mining, difficulty:', userDifficulty, 'challenge:', challenge.slice(0, 16));
        await invoke('start_mining', {
          address,
          challengeHex: challenge,
          difficulty: userDifficulty,
          blockTimestamp: latestBlock.timestamp,
          threads: threadCount,
          backend,
        });
      } catch (err) {
        console.error('[Mining] Failed to start native mining', err);
      }
      return;
    }

    // WASM path: generate random challenge client-side (like lithium-cli).
    // The contract accepts any 32-byte challenge — no need to use block hash.
    // This avoids restarting workers every ~6s block change.
    const randomBytes = new Uint8Array(32);
    crypto.getRandomValues(randomBytes);
    const challenge = Array.from(randomBytes, (b) => b.toString(16).padStart(2, '0')).join('');
    // Use local wall clock minus 5s safety margin (like lithium-cli)
    const timestamp = Math.floor(Date.now() / 1000) - 5;
    challengeRef.current = challenge;
    blockTimestampRef.current = timestamp;
    challengeCreatedAtRef.current = Date.now();

    try {
      console.log('[Mining] Starting WASM mining, difficulty:', userDifficulty, 'challenge:', challenge.slice(0, 16));
      if (!persistentWasmMiner) {
        const miner = new WasmMiner(threadCount);
        await miner.init();
        persistentWasmMiner = miner;
      }
      wasmMinerRef.current = persistentWasmMiner;
      persistentWasmMiner.start(challenge, userDifficulty);
    } catch (err) {
      console.error('[Mining] Failed to start WASM mining', err);
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
      // Use the challenge embedded in the proof (mined against), not the current one
      const proofChallenge = proof.challenge || challengeRef.current;
      const msg: ExecuteMsg = {
        submit_proof: {
          hash: proof.hash,
          nonce: Number(proof.nonce),
          miner_address: address,
          challenge: proofChallenge,
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

      // Build the execute message for sign()
      const encodeMsg = {
        typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract' as const,
        value: {
          sender: account.address,
          contract: LITIUM_MINE_CONTRACT,
          msg: toUtf8(JSON.stringify(msg)),
          funds: [],
        },
      };
      const fee = Soft3MessageFactory.fee(10);

      // Sign + broadcast with local sequence tracking
      const signAndBroadcastWithSeq = async () => {
        // Fetch sequence on first call (or after reset)
        if (!seqRef.current) {
          const { accountNumber, sequence } = await signingClient.getSequence(account.address);
          seqRef.current = { accountNumber, sequence };
          console.log('[Mining] Fetched sequence from chain:', sequence);
        }

        const signerData = {
          accountNumber: seqRef.current.accountNumber,
          sequence: seqRef.current.sequence,
          chainId: CHAIN_ID,
        };
        console.log('[Mining] Signing with sequence:', signerData.sequence);

        const txRaw = await signingClient.sign(account.address, [encodeMsg], fee, '', signerData);
        const txBytes = TxRaw.encode(txRaw).finish();
        const broadcastResult = await signingClient.broadcastTx(txBytes);

        // CyberClient.broadcastTx only does broadcastTxSync (CheckTx) — it does NOT
        // wait for block inclusion. We must poll getTx to verify DeliverTx succeeded.
        const txHash = broadcastResult.transactionHash;
        let deliverResult = await signingClient.getTx(txHash);
        if (!deliverResult) {
          // Wait for inclusion (up to ~15s, polling every 3s)
          for (let attempt = 0; attempt < 5 && !deliverResult; attempt++) {
            await new Promise<void>((r) => setTimeout(r, 3000));
            deliverResult = await signingClient.getTx(txHash);
          }
        }

        // Increment sequence — tx was broadcast (consumes sequence even if DeliverTx fails)
        seqRef.current.sequence += 1;

        if (!deliverResult) {
          throw new Error(`TX ${txHash} not found after broadcast — may have been dropped`);
        }
        if (deliverResult.code !== 0) {
          throw new Error(`Contract error: ${deliverResult.rawLog || `code ${deliverResult.code}`}`);
        }

        return {
          ...broadcastResult,
          code: deliverResult.code,
          height: deliverResult.height,
          events: deliverResult.events,
          rawLog: deliverResult.rawLog,
          gasUsed: deliverResult.gasUsed,
          gasWanted: deliverResult.gasWanted,
        };
      };

      let result;
      try {
        result = await signAndBroadcastWithSeq();
      } catch (executeErr: any) {
        const errText = normalizeErrorText(executeErr).toLowerCase();
        const kind = classifySubmitError(executeErr);

        // Sequence mismatch — re-fetch from chain and retry once
        if (/account sequence mismatch/.test(errText)) {
          console.log('[Mining] Sequence mismatch, re-fetching from chain...');
          seqRef.current = null;
          try {
            result = await signAndBroadcastWithSeq();
          } catch (retryErr) {
            if (isNative) {
              invoke('report_proof_failed').catch(() => {});
            }
            throw new Error(
              `Retry after sequence refresh failed: ${normalizeErrorText(retryErr)}`
            );
          }
        } else if (kind === 'account_not_found') {
          console.log('[Mining] Account not on chain, activating...');
          const activated = await activateAccount(account.address);
          if (activated) {
            console.log('[Mining] Account activated, waiting for tx inclusion...');
            await new Promise<void>((resolve) => {
              setTimeout(resolve, 7000);
            });
            console.log('[Mining] Retrying proof submission...');
            seqRef.current = null; // re-fetch after activation
            try {
              result = await signAndBroadcastWithSeq();
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
          // Reset sequence on any unknown error — next submit will re-fetch
          seqRef.current = null;
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
          p.hash === proof.hash && (p.status === 'submitted' || p.status === 'retrying' || !p.txHash)
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

    const now = Date.now();

    // --- Primary queue ---
    if (proofQueueRef.current.length > 0) {
      const elapsed_ = now - lastSubmitTimeRef.current;
      if (elapsed_ < SUBMIT_COOLDOWN_MS) {
        return;
      }

      // When offline, hold proofs — don't submit or discard
      if (!rpcOnline) {
        console.log('[Mining] Offline — holding', proofQueueRef.current.length, 'proofs in queue');
        return;
      }

      submittingRef.current = true;
      setSubmitting(true);

      const queue = [...proofQueueRef.current];
      proofQueueRef.current = [];

      if (queue.length === 0) {
        submittingRef.current = false;
        setSubmitting(false);
        return;
      }
      let bestIdx = 0;
      for (let i = 1; i < queue.length; i++) {
        if (queue[i].hash < queue[bestIdx].hash) {
          bestIdx = i;
        }
      }
      const best = queue[bestIdx];

      const discarded = queue.length - 1;
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
        lastSubmitTimeRef.current = Date.now();
        await submitSingleProof(best);
        consecutiveFailsRef.current = 0;
        resyncAfterProof();
      } catch (err: any) {
        console.error('[Mining] Submit failed:', err);
        const kind = classifySubmitError(err);

        if (kind === 'transport') {
          // Transport error → move to retry queue instead of marking failed
          consecutiveFailsRef.current += 1;
          console.log('[Mining] Transport error, consecutive fails:', consecutiveFailsRef.current);

          if (consecutiveFailsRef.current >= CONSECUTIVE_FAILS_THRESHOLD) {
            setRpcOnline(false);
            console.log('[Mining] RPC marked offline after', consecutiveFailsRef.current, 'consecutive failures');
          }

          if (retryQueueRef.current.length < MAX_RETRY_QUEUE) {
            retryQueueRef.current.push({
              proof: best,
              challenge: challengeRef.current,
              blockTimestamp: blockTimestampRef.current,
              attempts: 1,
              nextRetryAt: Date.now() + RETRY_BACKOFF_BASE,
            });
            // Update log entry to "retrying"
            setProofLog((prev) =>
              prev.map((p) =>
                p.hash === best.hash && p.status === 'submitted'
                  ? { ...p, error: 'Network error — will retry', status: 'retrying' }
                  : p
              )
            );
          } else {
            // Retry queue full — mark as failed
            setProofLog((prev) =>
              prev.map((p) =>
                p.hash === best.hash && p.status === 'submitted'
                  ? { ...p, error: 'Retry queue full', status: 'failed' }
                  : p
              )
            );
          }
        } else {
          // Contract/unknown error → permanent failure (no retry)
          setProofLog((prev) =>
            prev.map((p) =>
              p.hash === best.hash && p.status === 'submitted'
                ? { ...p, error: err?.message || 'Failed', status: 'failed' }
                : p
            )
          );
          resyncAfterProof();
        }
      } finally {
        submittingRef.current = false;
        setSubmitting(false);
      }
    }

    // --- Retry queue --- (only when online and not already submitting)
    if (!rpcOnline || submittingRef.current || retryQueueRef.current.length === 0) {
      return;
    }

    const retryNow = Date.now();
    const ready = retryQueueRef.current.findIndex(
      (r) => r.nextRetryAt <= retryNow
    );
    if (ready === -1) return;

    const entry = retryQueueRef.current[ready];
    retryQueueRef.current.splice(ready, 1);

    // Check if challenge is still current
    if (entry.challenge !== challengeRef.current) {
      console.log('[Mining] Retry discarded — stale challenge');
      setProofLog((prev) =>
        prev.map((p) =>
          p.hash === entry.proof.hash && p.status === 'retrying'
            ? { ...p, error: 'Stale challenge', status: 'failed' }
            : p
        )
      );
      return;
    }

    submittingRef.current = true;
    setSubmitting(true);
    console.log('[Mining] Retrying proof:', entry.proof.hash.slice(0, 16), 'attempt:', entry.attempts + 1);

    try {
      lastSubmitTimeRef.current = Date.now();
      await submitSingleProof(entry.proof);
      consecutiveFailsRef.current = 0;
      resyncAfterProof();
      // submitSingleProof already updates the log entry to 'success'
    } catch (err: any) {
      const kind = classifySubmitError(err);

      if (kind === 'transport' && entry.attempts < MAX_RETRY_ATTEMPTS) {
        consecutiveFailsRef.current += 1;
        if (consecutiveFailsRef.current >= CONSECUTIVE_FAILS_THRESHOLD) {
          setRpcOnline(false);
        }
        // Put back with increased backoff
        retryQueueRef.current.push({
          ...entry,
          attempts: entry.attempts + 1,
          nextRetryAt: Date.now() + RETRY_BACKOFF_BASE * Math.pow(2, entry.attempts),
        });
      } else {
        // Max retries exceeded or non-transport error
        setProofLog((prev) =>
          prev.map((p) =>
            p.hash === entry.proof.hash && p.status === 'retrying'
              ? { ...p, error: err?.message || 'Failed after retries', status: 'failed' }
              : p
          )
        );
        resyncAfterProof();
      }
    } finally {
      submittingRef.current = false;
      setSubmitting(false);
    }
  }, [submitSingleProof, rpcOnline, resyncAfterProof]);

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
            // Tag each proof with the challenge it was mined against
            const currentChallenge = challengeRef.current;
            const tagged = proofs.map((p) => ({ ...p, challenge: p.challenge || currentChallenge }));
            const MAX_QUEUE = 50;
            if (proofQueueRef.current.length < MAX_QUEUE) {
              proofQueueRef.current.push(...tagged.slice(0, MAX_QUEUE - proofQueueRef.current.length));
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

  // Native: hot-swap challenge when a new block arrives while mining is active.
  useEffect(() => {
    if (!autoMining || !isNative || !latestBlock) return;
    const newChallenge = latestBlock.blockHash;
    if (newChallenge === challengeRef.current) return;

    console.log('[Mining] Block changed, updating native challenge:', newChallenge.slice(0, 16));
    challengeRef.current = newChallenge;
    blockTimestampRef.current = latestBlock.timestamp;

    invoke('update_challenge', {
      challengeHex: newChallenge,
      blockTimestamp: latestBlock.timestamp,
    }).catch((err) => console.warn('[Mining] update_challenge failed:', err));
  }, [autoMining, latestBlock, isNative]);

  // WASM: rotate challenge before it expires (max_proof_age).
  // Unlike native which uses block hashes, WASM generates random challenges
  // and mines continuously. Rotate at 80% of max_proof_age to leave submission margin.
  useEffect(() => {
    if (!autoMining || isNative || !persistentWasmMiner) return;
    const maxAge = config?.max_proof_age ?? 3600;
    const rotateAfterMs = maxAge * 0.8 * 1000; // 80% of max age

    const timer = setInterval(() => {
      const elapsed = Date.now() - challengeCreatedAtRef.current;
      if (elapsed >= rotateAfterMs) {
        const randomBytes = new Uint8Array(32);
        crypto.getRandomValues(randomBytes);
        const newChallenge = Array.from(randomBytes, (b) => b.toString(16).padStart(2, '0')).join('');
        const timestamp = Math.floor(Date.now() / 1000) - 5;
        console.log('[Mining] Rotating challenge (age:', Math.round(elapsed / 1000), 's), new:', newChallenge.slice(0, 16));
        challengeRef.current = newChallenge;
        blockTimestampRef.current = timestamp;
        challengeCreatedAtRef.current = Date.now();
        persistentWasmMiner.start(newChallenge, userDifficulty);
      }
    }, 30_000); // Check every 30s

    return () => clearInterval(timer);
  }, [autoMining, isNative, config?.max_proof_age, userDifficulty]);

  // Auto-adjust difficulty if contract min_difficulty increases above user setting
  useEffect(() => {
    if (config && config.min_difficulty > userDifficulty) {
      console.log('[Mining] Config min_difficulty increased, adjusting:', config.min_difficulty);
      setUserDifficulty(config.min_difficulty);
    }
  }, [config, userDifficulty]);

  const handleStartMining = useCallback(async () => {
    // If genesis is in the future, enter countdown mode instead of mining
    console.log('[Mining][start] genesis check:', {
      genesis_time: config?.genesis_time,
      now: Math.floor(Date.now() / 1000),
      inFuture: config?.genesis_time ? config.genesis_time > Math.floor(Date.now() / 1000) : 'no config',
    });
    if (config?.genesis_time && config.genesis_time > Math.floor(Date.now() / 1000)) {
      setCountdownMode(true);
      return;
    }
    miningAddressRef.current = address;
    setAutoMining(true);
    await startMiningRound();
    startPolling();
  }, [startMiningRound, startPolling, address, config?.genesis_time]);

  const handleStopMining = useCallback(async () => {
    // Guard: ignore click events from the same frame as mount (stale click-through
    // from previous page's ActionBar back button hitting our Stop button).
    if (!stopReadyRef.current) {
      console.log('[Mining][stop] IGNORED — stale click before first frame');
      return;
    }
    console.log('[Mining][stop] handleStopMining called', new Error().stack?.split('\n').slice(1, 4).join(' <- '));
    setCountdownMode(false);
    setAutoMining(false);
    retryQueueRef.current = [];
    consecutiveFailsRef.current = 0;
    setRpcOnline(true);
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
    proofQueueRef.current = [];
    // Resolve any "submitted" proofs that are still pending (tx may still confirm in background)
    setProofLog((prev) =>
      prev.map((p) =>
        p.status === 'submitted' && !p.txHash
          ? { ...p, status: 'failed' as const, error: 'Mining stopped' }
          : p
      )
    );
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

  // TESTING: Countdown auto-start — remove after testing phase
  useEffect(() => {
    if (!countdownMode || countdownSeconds > 0) return;
    // Double-check genesis is actually in the past (guards against stale-0 race)
    if (config?.genesis_time && config.genesis_time > Math.floor(Date.now() / 1000)) return;
    console.log('[Mining] Countdown reached zero — launching mining');
    setCountdownMode(false);
    miningAddressRef.current = address;
    setAutoMining(true);
    startMiningRound().then(() => startPolling());
  }, [countdownMode, countdownSeconds, address, startMiningRound, startPolling, config?.genesis_time]);

  const handleCopyAddress = useCallback(() => {
    if (address) {
      navigator.clipboard.writeText(address);
    }
  }, [address]);

  // Build report params from current state (sync — no Tauri invoke)
  const getReportParams = useCallback(
    (tauriStatus: Record<string, unknown> | null = null, tauriParams: Record<string, unknown> | null = null): ReportParams => ({
      isNative,
      address,
      referrer,
      liBalance,
      autoMining,
      userDifficulty,
      minDifficulty,
      threadCount,
      backend,
      availableBackends,
      hashrate,
      totalHashes: miningStatus?.total_hashes ?? 0,
      elapsed,
      pendingProofs: miningStatus?.pending_proofs ?? 0,
      sessionLiMined,
      tauriStatus,
      tauriParams,
      latestBlock: latestBlock
        ? { height: latestBlock.height, blockHash: latestBlock.blockHash, timestamp: latestBlock.timestamp }
        : null,
      wsConnected,
      samples,
      config,
      windowStatus,
      uniqueMiners,
      totalProofs,
      avgDifficulty,
      dRate,
      similarDevices,
      windowEntries,
      baseRate,
      rewardPerProof,
      grossRewardPerProof,
      estimatedLiPerHour,
      rpcOnline,
      retryQueueSize: retryQueueRef.current.length,
      consecutiveFails: consecutiveFailsRef.current,
      proofLog,
      sessionStart,
    }),
    [
      address, referrer, liBalance, autoMining, userDifficulty, minDifficulty,
      threadCount, backend, availableBackends, hashrate, miningStatus, elapsed,
      sessionLiMined, latestBlock, wsConnected, samples, config, windowStatus,
      uniqueMiners, totalProofs, avgDifficulty, dRate,
      similarDevices, windowEntries, baseRate, rewardPerProof,
      grossRewardPerProof, estimatedLiPerHour, proofLog, isNative, rpcOnline,
      sessionStart,
    ]
  );

  // Auto-save mining logs to disk (Tauri only)
  const { lastSavePath, saveToDailyFile, clearAllLogs, revealInFinder } = useAutoSaveLogs({
    enabled: isNative && autoMining,
    sessionStart,
    buildReport: useCallback(
      () => buildMiningReport(getReportParams()),
      [getReportParams]
    ),
  });

  // Clear stale local data when genesis hasn't started (post-reset)
  useEffect(() => {
    if (!genesisInFuture) return;
    localStorage.removeItem(PROOF_LOG_KEY);
    localStorage.removeItem(SESSION_LI_KEY);
    localStorage.removeItem(MINING_ACTIVE_KEY);
    localStorage.removeItem(MINING_ADDRESS_KEY);
    setProofLog([]);
    setSessionLiMined(0);
    setAutoMining(false);
    clearAllLogs();
  }, [genesisInFuture, clearAllLogs]);

  const handleExportLogs = useCallback(async () => {
    // Tauri: save to daily sessions file (same format as auto-save)
    if (isNative) {
      const saved = await saveToDailyFile();
      if (saved) {
        console.log('[Mining] Exported to daily file:', saved);
      }
      return;
    }

    // Web: browser file download
    const report = buildMiningReport(getReportParams());
    const text = JSON.stringify(report, null, 2);
    const filename = `mining-log-${Date.now()}.json`;
    const blob = new Blob([text], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }, [isNative, getReportParams, saveToDailyFile]);

  // PoW share from on-chain state: 1 - S^alpha (100% when nothing staked)
  const powSharePercent = (powShare * 100).toFixed(1);
  // Miner's network share
  const userDRate = hashrate > 0 && userDifficulty > 0
    ? userDifficulty * (hashrate / Math.pow(2, userDifficulty))
    : 0;
  const networkSharePercent = userDRate > 0 && dRate > 0
    ? Math.min((userDRate / dRate) * 100, 100)
    : 0;

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
              {genesisInFuture
                ? <Pill color="yellow" text={countdownMode ? 'Launching' : 'Countdown'} />
                : autoMining && !rpcOnline
                  ? <Pill color="yellow" text="Offline" />
                  : <Pill color="black" text={autoMining ? 'Mining' : 'Idle'} />}
            </div>
          </div>

          {/* Auto-save info (Tauri only) */}
          {lastSavePath && (
            <div className={styles.autoSaveRow}>
              <span className={styles.autoSavePath} title={lastSavePath}>
                Auto-save: {lastSavePath.replace(/^.*\/Library\/Logs\//, '~/Library/Logs/')}
              </span>
              <button
                type="button"
                className={styles.copyBtn}
                onClick={revealInFinder}
                title="Reveal in Finder"
              >
                Reveal
              </button>
            </div>
          )}

          {/* Config panel (collapsible) */}
          <ConfigPanel open={configOpen} config={config} onConfigUpdated={refetchConfig} />

          {/* Desktop download CTA (web only — first thing users see) */}
          {!isNative && <DownloadSection address={address} accountName={defaultAccount?.name || undefined} />}

          {/* Hero: big hashrate + sparkline */}
          <HashrateHero
            hashrate={hashrate}
            isActive={autoMining}
            samples={samples}
            sessionAvg={elapsed > 0 ? (miningStatus?.total_hashes ?? 0) / elapsed : undefined}
            countdown={countdownSeconds > 0 ? countdownSeconds : undefined}
            genesisTimeSec={genesisInFuture ? config?.genesis_time : undefined}
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

          {/* Reward — per-proof math chain */}
          <div className={styles.sectionBox}>
            <span className={styles.sectionTitle}>Reward</span>
            <div className={styles.statsGrid}>
              <StatCard label="Gross / proof" value={compactLi(grossRewardPerProof)} suffix="LI" />
              <StatCard label="PoW share" value={`${powSharePercent}%`} />
              <StatCard label="Referral / pool cut" value="10%" />
              <StatCard label="Net / proof" value={compactLi(rewardPerProof)} suffix="LI" />
            </div>
          </div>

          {/* Network info */}
          <div className={styles.sectionBox}>
            <div className={styles.networkHeader}>
              <span className={styles.sectionTitle}>Network</span>
              <span className={styles.refreshBadge}>
                {wsConnected ? 'live' : 'reconnecting...'}
              </span>
            </div>
            <div className={styles.statsGrid}>
              <StatCard
                label="Your share"
                value={networkSharePercent > 0 ? `${networkSharePercent.toFixed(1)}%` : '\u2014'}
              />
              <StatCard label="Window" value={`${windowEntries} / ${windowSize || '...'}`}>
                {windowSize > 0 && (
                  <div className={styles.windowProgress}>
                    <div
                      className={styles.windowProgressFill}
                      style={{ width: `${Math.min((windowEntries / windowSize) * 100, 100)}%` }}
                    />
                  </div>
                )}
              </StatCard>
              {totalMinted && (
                <StatCard
                  label="Supply"
                  value={`${formatLi(totalMinted.total_minted)} / ${formatLi(totalMinted.supply_cap)}`}
                  suffix="LI"
                />
              )}
              <StatCard
                label="Active miners"
                value={uniqueMiners > 0 ? uniqueMiners : '...'}
              />
              <StatCard
                label="Avg difficulty"
                value={avgDifficulty > 0 ? avgDifficulty.toFixed(1) : '...'}
                suffix="bits"
              />
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
            const retrying = proofLog.filter((p) => p.status === 'retrying').length;
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
                  {retrying > 0 && <StatCard label="Retrying" value={retrying} />}
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
        maxThreads={isNative ? Math.max(1, cpuCores - 1) : Math.max(1, Math.floor(cpuCores / 2))}
        isNative={isNative}
        countdownMode={countdownMode}
        countdownSeconds={countdownSeconds}
      />
    </MainContainer>
  );
}

export default Mining;

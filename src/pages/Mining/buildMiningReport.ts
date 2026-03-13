import type { WindowStatusResponse, ConfigResponse } from 'src/generated/lithium/LitiumMine.types';

type ProofLogEntry = {
  hash: string;
  nonce: number;
  txHash?: string;
  error?: string;
  status?: 'submitted' | 'pending' | 'success' | 'failed' | 'retrying';
  timestamp: number;
};

export type ReportParams = {
  isNative: boolean;
  address: string | undefined;
  referrer: string | null;
  liBalance: number;
  autoMining: boolean;
  userDifficulty: number;
  minDifficulty: number;
  threadCount: number;
  backend: string;
  availableBackends: string[];
  hashrate: number;
  totalHashes: number;
  elapsed: number;
  pendingProofs: number;
  sessionLiMined: number;
  tauriStatus: Record<string, unknown> | null;
  tauriParams: Record<string, unknown> | null;
  latestBlock: { height: number; blockHash: string; timestamp: number } | null;
  wsConnected: boolean;
  samples: number[];
  config: ConfigResponse | undefined;
  windowStatus: WindowStatusResponse | undefined;
  uniqueMiners: number;
  totalProofs: number;
  avgDifficulty: number;
  dRate: number;
  similarDevices: number;
  windowEntries: number;
  baseRate: number;
  rewardPerProof: number;
  grossRewardPerProof: number;
  estimatedLiPerHour: number;
  rpcOnline: boolean;
  retryQueueSize: number;
  consecutiveFails: number;
  proofLog: ProofLogEntry[];
  sessionStart: number;
  actionLog?: { action: string; detail?: string; timestamp: number; result?: string; error?: string }[];
};

export function buildMiningReport(params: ReportParams): object {
  const sessionLog = params.proofLog.filter(
    (p) => p.timestamp >= params.sessionStart
  );
  const accepted = sessionLog.filter((p) => p.status === 'success').length;
  const failed = sessionLog.filter(
    (p) => p.status === 'failed' || (p.error && !p.status)
  ).length;
  const pending = sessionLog.filter(
    (p) => p.status === 'submitted' || p.status === 'pending'
  ).length;
  const total = accepted + failed;

  const nav = navigator as Navigator & { deviceMemory?: number };
  const device = {
    cpu_cores: navigator.hardwareConcurrency || null,
    device_memory_gb: nav.deviceMemory || null,
    platform: navigator.platform || null,
    max_touch_points: navigator.maxTouchPoints,
  };

  return {
    exported_at: new Date().toISOString(),
    user_agent: navigator.userAgent,
    platform: params.isNative ? 'tauri' : 'web',
    address: params.address || null,
    referrer: params.referrer || null,
    li_balance: params.liBalance,
    device,
    mining: {
      active: params.autoMining,
      difficulty: params.userDifficulty,
      min_difficulty: params.minDifficulty,
      threads: params.threadCount,
      backend: params.backend,
      available_backends: params.availableBackends,
      hashrate: params.hashrate,
      total_hashes: params.totalHashes,
      elapsed_secs: params.elapsed,
      pending_proofs: params.pendingProofs,
      session_li_mined: params.sessionLiMined,
      batch_count: (params.tauriStatus?.batch_count as number) ?? null,
      avg_batch_ms: (params.tauriStatus?.avg_batch_ms as number) ?? null,
      proofs_submitted:
        (params.tauriStatus?.proofs_submitted as number) ?? null,
      proofs_failed: (params.tauriStatus?.proofs_failed as number) ?? null,
    },
    uhash_params: params.tauriParams
      ? {
          chains: params.tauriParams.chains,
          scratchpad_kb: params.tauriParams.scratchpad_kb,
          total_mb: params.tauriParams.total_mb,
          rounds: params.tauriParams.rounds,
          block_size: params.tauriParams.block_size,
        }
      : null,
    block: params.latestBlock
      ? {
          height: params.latestBlock.height,
          hash: params.latestBlock.blockHash,
          timestamp: params.latestBlock.timestamp,
        }
      : null,
    ws_connected: params.wsConnected,
    hashrate_samples: params.samples,
    contract_config: params.config || null,
    window_status: params.windowStatus || null,
    network: {
      unique_miners: params.uniqueMiners,
      total_proofs: params.totalProofs,
      avg_difficulty: params.avgDifficulty,
      d_rate: params.dRate,
      similar_devices: params.similarDevices,
      window_proof_count: params.windowEntries,
      base_rate: params.baseRate,
    },
    reward_estimate: {
      per_proof: params.rewardPerProof,
      gross_per_proof: params.grossRewardPerProof,
      li_per_hour: params.estimatedLiPerHour,
    },
    connection: {
      rpc_online: params.rpcOnline,
      retry_queue_size: params.retryQueueSize,
      consecutive_fails: params.consecutiveFails,
    },
    proof_summary: {
      total: sessionLog.length,
      accepted,
      failed,
      pending,
      retrying: sessionLog.filter((p) => p.status === 'retrying').length,
      success_rate:
        total > 0 ? `${((accepted / total) * 100).toFixed(1)}%` : null,
    },
    proof_log: sessionLog,
    action_log: params.actionLog || [],
  };
}

import useQueryContract from 'src/hooks/contract/useQueryContract';
import { LITIUM_MINE_CONTRACT } from 'src/constants/mining';

type DifficultyResponse = {
  current: number;
  min_profitable: number;
  window_proof_count: number;
  window_total_work: string; // Uint128 comes as string
};

type ConfigResponse = {
  period_duration: number;
  difficulty: number;
  base_reward: string;
  alpha_micros: number;
  lithium_epoch_duration_blocks: number;
  target_proofs_per_window: number;
};

function usePeerEstimate(localHashrate: number) {
  const { data, loading, dataUpdatedAt } = useQueryContract(LITIUM_MINE_CONTRACT, {
    difficulty: {},
  });

  const { data: configData } = useQueryContract(LITIUM_MINE_CONTRACT, {
    config: {},
  });

  const resp = data as DifficultyResponse | undefined;
  const config = configData as ConfigResponse | undefined;

  const windowDuration = config?.period_duration ?? 600;
  const windowProofCount = resp?.window_proof_count ?? 0;
  const diffBits = resp?.current ?? 0;

  // Use BigInt to avoid precision loss on large Uint128 values
  let rawHashrate = 0;
  let windowWork = 0;
  if (resp?.window_total_work) {
    try {
      const workBig = BigInt(resp.window_total_work);
      windowWork = Number(workBig);
      rawHashrate =
        Number(workBig * 1000n / BigInt(windowDuration)) / 1000;
    } catch {
      windowWork = Number(resp.window_total_work);
      rawHashrate = windowWork / windowDuration;
    }
  }

  // Adjust for cherry-picking: miners submit only the best proof per
  // cooldown period, so submitted proofs have far more leading zeros than
  // the minimum difficulty requires. The contract records work as
  // 2^(actual_leading_zeros), inflating apparent work.
  //
  // cherry_factor = avgWorkPerProof / minWork
  //   where minWork = 2^difficulty (work at exactly minimum difficulty)
  // adjustedHashrate = rawHashrate / cherry_factor
  const minWork = 2 ** diffBits;
  const avgWorkPerProof =
    windowProofCount > 0 ? windowWork / windowProofCount : 0;
  const cherryFactor =
    minWork > 0 && avgWorkPerProof > minWork
      ? avgWorkPerProof / minWork
      : 1;
  const networkHashrate =
    cherryFactor > 1 ? rawHashrate / cherryFactor : rawHashrate;

  const similarDevices =
    localHashrate > 0
      ? Math.max(1, Math.round(networkHashrate / localHashrate))
      : 0;

  return {
    networkHashrate,
    similarDevices,
    windowProofCount,
    minProfitable: resp?.min_profitable ?? 0,
    loading,
    dataUpdatedAt,
  };
}

export default usePeerEstimate;

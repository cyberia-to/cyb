import useQueryContract from 'src/hooks/contract/useQueryContract';
import { LITIUM_MINE_CONTRACT, SUBMIT_COOLDOWN_MS } from 'src/constants/mining';
import type {
  RewardCalculationResponse,
  EmissionInfoResponse,
} from 'src/generated/lithium/LitiumMine.types';

function useRewardEstimate(
  difficulty: number | undefined,
  hashrate: number,
  emission?: EmissionInfoResponse
) {
  const { data, refetch } = useQueryContract(
    LITIUM_MINE_CONTRACT,
    difficulty !== undefined && difficulty > 0
      ? { calculate_reward: { difficulty_bits: difficulty } }
      : { config: {} } // dummy query when no difficulty
  );

  const rewardResp = data as RewardCalculationResponse | undefined;

  // Contract returns gross_reward = base_rate * d — the TOTAL reward before split.
  // The actual split in execute_submit_proof:
  //   staking_reward = gross * S^alpha
  //   pow_reward     = gross - staking_reward
  //   referral       = pow_reward * 10%
  //   miner_reward   = pow_reward - referral
  const grossReward =
    difficulty !== undefined && rewardResp
      ? Number(rewardResp.gross_reward ?? 0) / 1_000_000
      : 0;

  // Compute PoW share from emission_info: powShare = mining_rate / gross_rate
  // This equals (1 - S^alpha) — the fraction of reward going to PoW (mining + referral)
  let powShare = 1; // default: assume 100% PoW if no emission data
  if (emission) {
    const grossRate = Number(emission.gross_rate);
    if (grossRate > 0) {
      powShare = Number(emission.mining_rate) / grossRate;
    }
  }

  const referralCut = 0.1;
  const minerReward = grossReward * powShare * (1 - referralCut);

  // Cap estimated proofs/hr by the submission cooldown
  const maxProofsPerHour = 3600 / (SUBMIT_COOLDOWN_MS / 1000);
  const theoreticalProofsPerHour =
    difficulty !== undefined && difficulty > 0 && hashrate > 0
      ? (hashrate * 3600) / 2 ** difficulty
      : 0;
  const effectiveProofsPerHour = Math.min(
    theoreticalProofsPerHour,
    maxProofsPerHour
  );
  const estimatedLiPerHour =
    minerReward > 0 ? effectiveProofsPerHour * minerReward : 0;

  return {
    rewardPerProof: minerReward,
    grossRewardPerProof: grossReward,
    estimatedLiPerHour,
    loading: !data,
    refetch,
  };
}

export default useRewardEstimate;

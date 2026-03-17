import useQueryContract from 'src/hooks/contract/useQueryContract';
import { LITIUM_MINE_CONTRACT, SUBMIT_COOLDOWN_MS } from 'src/constants/mining';
import type { RewardCalculationResponse } from 'src/generated/lithium/LitiumMine.types';

function useRewardEstimate(
  difficulty: number | undefined,
  hashrate: number,
  powShare: number
) {
  const { data, refetch } = useQueryContract(
    LITIUM_MINE_CONTRACT,
    difficulty !== undefined && difficulty > 0
      ? { calculate_reward: { difficulty_bits: difficulty } }
      : { config: {} } // dummy query when no difficulty
  );

  const rewardResp = data as RewardCalculationResponse | undefined;

  // Contract returns gross_reward = base_rate * d — the TOTAL reward before split.
  // The actual split in execute_submit_proof (spec §5):
  //   referral       = gross * 10%
  //   post_referral  = gross * 90%
  //   staking_reward = post_referral * S^alpha
  //   miner_reward   = post_referral - staking_reward = gross * 0.9 * (1 - S^alpha)
  const grossReward =
    difficulty !== undefined && rewardResp
      ? Number(rewardResp.gross_reward ?? 0) / 1_000_000
      : 0;

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

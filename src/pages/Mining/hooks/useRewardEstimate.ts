import useQueryContract from 'src/hooks/contract/useQueryContract';
import { UHASH_CONTRACT } from 'src/constants/mining';

function useRewardEstimate(
  difficulty: number | undefined,
  hashrate: number
) {
  const { data } = useQueryContract(
    UHASH_CONTRACT,
    difficulty !== undefined
      ? { calculate_reward: { difficulty_bits: difficulty } }
      : { epoch_status: {} } // dummy query when no difficulty
  );

  const grossReward =
    difficulty !== undefined && data
      ? Number((data as any).gross_reward ?? 0) / 1_000_000
      : 0;

  // Lithium v1 reward split: mining gets (1000 - alpha_permille) / 1000
  // and of the mining portion, 10% goes to referral if referrer is set.
  // Show the miner-received amount (worst case with referral deduction).
  const alphaPermille = (data as any)?.alpha_permille ?? 500; // default 50% to staking
  const miningFraction = (1000 - Number(alphaPermille)) / 1000;
  const referralCut = 0.1; // 10% of mining portion
  const minerReward = grossReward * miningFraction * (1 - referralCut);

  const estimatedLiPerHour =
    difficulty !== undefined && difficulty > 0 && minerReward > 0
      ? (hashrate * 3600 * minerReward) / 2 ** difficulty
      : 0;

  return {
    rewardPerProof: minerReward,
    grossRewardPerProof: grossReward,
    miningFraction,
    estimatedLiPerHour,
    loading: !data,
  };
}

export default useRewardEstimate;

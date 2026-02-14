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
      : { seed: {} } // dummy query when no difficulty
  );

  const grossReward =
    difficulty !== undefined && data
      ? Number((data as any).gross_reward ?? 0) / 1_000_000
      : 0;

  const estimatedLiPerHour =
    difficulty !== undefined && difficulty > 0 && grossReward > 0
      ? (hashrate * 3600 * grossReward) / 2 ** difficulty
      : 0;

  return {
    rewardPerProof: grossReward,
    estimatedLiPerHour,
    loading: !data,
  };
}

export default useRewardEstimate;

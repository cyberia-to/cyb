import useQueryContract from 'src/hooks/contract/useQueryContract';
import { LITIUM_MINE_CONTRACT, SUBMIT_COOLDOWN_MS } from 'src/constants/mining';

function useRewardEstimate(
  difficulty: number | undefined,
  hashrate: number
) {
  const { data } = useQueryContract(
    LITIUM_MINE_CONTRACT,
    difficulty !== undefined
      ? { calculate_reward: { difficulty_bits: difficulty } }
      : { epoch_status: {} } // dummy query when no difficulty
  );

  // Query config for alpha_micros (staking/mining split).
  // calculate_reward response does NOT include alpha_micros,
  // so we must get it from config separately.
  const { data: configData } = useQueryContract(LITIUM_MINE_CONTRACT, {
    config: {},
  });

  const grossReward =
    difficulty !== undefined && data
      ? Number((data as any).gross_reward ?? 0) / 1_000_000
      : 0;

  // Lithium reward split: mining gets (1_000_000 - alpha_micros) / 1_000_000
  // and of the mining portion, 10% goes to referral if referrer is set.
  // Show the miner-received amount (worst case with referral deduction).
  const alphaMicros = (configData as any)?.alpha_micros ?? 0;
  const miningFraction = (1_000_000 - Number(alphaMicros)) / 1_000_000;
  const referralCut = 0.1; // 10% of mining portion
  const minerReward = grossReward * miningFraction * (1 - referralCut);

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
    miningFraction,
    estimatedLiPerHour,
    loading: !data,
  };
}

export default useRewardEstimate;

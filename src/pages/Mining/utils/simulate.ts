/**
 * Simulation engine for Lithium mining parameters.
 * Pure function: SimParams → SimResult.
 */

import {
  totalRateAtomicPerSecond,
  totalEmittedAtomic,
  componentRates,
  atomicToHuman,
  LI_TOTAL_SUPPLY_ATOMIC,
  SECONDS_PER_DAY,
  type ComponentRate,
} from './emission';

export type SimParams = {
  elapsedDays: number; // 0 → 7300 (20 years)
  networkHashrate: number; // H/s
  yourHashrate: number; // H/s (personal)
  stakedPercent: number; // 0-100
  dailyTransfers: number; // for burn projection
  difficultyBits: number; // independent input
  targetSolutions: number; // proofs per epoch target
  epochSeconds: number; // epoch duration
  liPrice: number; // USD per LI
};

export type SimResult = {
  components: ComponentRate[];
  emissionPerEpoch: number;
  emissionPerSecond: number;
  miningPercent: number;
  stakingPercent: number;
  referralPercent: number;
  alpha: number;
  difficultyBits: number;
  equilibriumDifficulty: number;
  rewardPerProof: number;
  totalMinted: number;
  mintedPercent: number;
  dailyBurn: number;
  netDailyInflation: number;
  // Per-split emission (human LI per epoch)
  miningEmissionPerEpoch: number;
  stakingEmissionPerEpoch: number;
  referralEmissionPerEpoch: number;
  // Personal stats
  yourProofsPerDay: number;
  yourLiPerDay: number;
  yourLiPerMonth: number;
  timePerProofSeconds: number;
  yourNetworkSharePercent: number;
  yourDailyUsd: number;
  yourMonthlyUsd: number;
};

const REFERRAL_SHARE = 0.1; // 10%
const BURN_PER_TRANSFER = 0.5;

function computeSplit(alpha: number) {
  const referralPercent = REFERRAL_SHARE * 100;
  const nonRef = 1 - REFERRAL_SHARE;
  const stakingFraction = nonRef * (alpha / 2);
  const miningFraction = nonRef - stakingFraction;
  return {
    miningPercent: miningFraction * 100,
    stakingPercent: stakingFraction * 100,
    referralPercent,
  };
}

function calcEquilibriumDifficulty(
  networkHashrate: number,
  epochSeconds: number,
  targetSolutions: number
): number {
  if (networkHashrate <= 0 || targetSolutions <= 0) return 0;
  const hashesPerEpoch = networkHashrate * epochSeconds;
  if (hashesPerEpoch <= targetSolutions) return 0;
  return Math.max(0, Math.floor(Math.log2(hashesPerEpoch / targetSolutions)));
}

export function simulate(params: SimParams): SimResult {
  const elapsedSeconds = params.elapsedDays * SECONDS_PER_DAY;

  const components = componentRates(elapsedSeconds);
  const totalRateAtomic = totalRateAtomicPerSecond(elapsedSeconds);
  const totalMintedAtomic = totalEmittedAtomic(elapsedSeconds);

  const emissionPerSecond = atomicToHuman(totalRateAtomic);
  const emissionPerEpoch = emissionPerSecond * params.epochSeconds;

  const alpha = Math.max(0, Math.min(1, params.stakedPercent / 100));
  const { miningPercent, stakingPercent, referralPercent } =
    computeSplit(alpha);

  const difficultyBits = params.difficultyBits;
  const equilibriumDifficulty = calcEquilibriumDifficulty(
    params.networkHashrate,
    params.epochSeconds,
    params.targetSolutions
  );

  // Per-split emission
  const miningEmissionPerEpoch = emissionPerEpoch * (miningPercent / 100);
  const stakingEmissionPerEpoch = emissionPerEpoch * (stakingPercent / 100);
  const referralEmissionPerEpoch = emissionPerEpoch * (referralPercent / 100);

  // Reward per proof = epoch_emission * mining_fraction / target_solutions
  const rewardPerProof =
    params.targetSolutions > 0
      ? miningEmissionPerEpoch / params.targetSolutions
      : 0;

  const totalMinted = atomicToHuman(totalMintedAtomic);
  const mintedPercent = (totalMintedAtomic / LI_TOTAL_SUPPLY_ATOMIC) * 100;

  const dailyBurn = params.dailyTransfers * BURN_PER_TRANSFER;
  const dailyEmission = emissionPerSecond * SECONDS_PER_DAY;
  const netDailyInflation = dailyEmission - dailyBurn;

  // Personal stats
  const hashesPerProof = difficultyBits > 0 ? 2 ** difficultyBits : 1;
  const yourProofsPerSecond =
    hashesPerProof > 0 ? params.yourHashrate / hashesPerProof : 0;
  const yourProofsPerDay = yourProofsPerSecond * SECONDS_PER_DAY;
  const yourLiPerDay = yourProofsPerDay * rewardPerProof;
  const yourLiPerMonth = yourLiPerDay * 30;
  const timePerProofSeconds =
    yourProofsPerSecond > 0 ? 1 / yourProofsPerSecond : Infinity;
  const yourNetworkSharePercent =
    params.networkHashrate > 0
      ? (params.yourHashrate / params.networkHashrate) * 100
      : 0;
  const yourDailyUsd = yourLiPerDay * params.liPrice;
  const yourMonthlyUsd = yourLiPerMonth * params.liPrice;

  return {
    components,
    emissionPerEpoch,
    emissionPerSecond,
    miningPercent,
    stakingPercent,
    referralPercent,
    alpha,
    difficultyBits,
    equilibriumDifficulty,
    rewardPerProof,
    totalMinted,
    mintedPercent,
    dailyBurn,
    netDailyInflation,
    miningEmissionPerEpoch,
    stakingEmissionPerEpoch,
    referralEmissionPerEpoch,
    yourProofsPerDay,
    yourLiPerDay,
    yourLiPerMonth,
    timePerProofSeconds,
    yourNetworkSharePercent,
    yourDailyUsd,
    yourMonthlyUsd,
  };
}

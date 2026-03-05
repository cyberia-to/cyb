/**
 * Simulation engine for Lithium mining — adaptive hybrid economics.
 * Pure function: SimParams → SimResult.
 *
 * In the new model:
 * - No epochs — continuous sliding window
 * - Client-chosen difficulty d → reward = base_rate * d
 * - base_rate = (G * R_pow_share) / D_rate
 * - G = emission_rate + fee_rate * (1 - beta)
 * - R_pow_share = 1 - S^alpha
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
  dailyTransfers: number; // for fee projection
  difficultyBits: number; // client-chosen difficulty
  liPrice: number; // USD per LI
  alpha: number; // PID alpha [0.3, 0.7]
  beta: number; // PID beta [0.0, 0.9]
  gasCostUboot: number; // gas cost per proof in uboot
};

export type SimResult = {
  components: ComponentRate[];
  emissionPerHour: number;
  emissionPerSecond: number;
  miningPercent: number;
  stakingPercent: number;
  referralPercent: number;
  alpha: number;
  difficultyBits: number;
  baseRate: number;
  rewardPerProof: number;
  totalMinted: number;
  mintedPercent: number;
  dailyFeeBurn: number;
  netDailyInflation: number;
  gasCostPerProofLi: number;
  netRewardPerProof: number;
  // Per-split emission (human LI per hour)
  miningEmissionPerHour: number;
  stakingEmissionPerHour: number;
  referralEmissionPerHour: number;
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
const FEE_PER_TRANSFER = 0.01; // 1% fee per transfer
const AVERAGE_TRANSFER_AMOUNT = 50; // average LI per transfer (assumption)

function computeSplit(stakedFraction: number, alpha: number) {
  // R_pow_share = 1 - S^alpha (PoW share)
  // R_pos_share = S^alpha (PoS share)
  const sAlpha = stakedFraction > 0 && alpha > 0 ? Math.pow(stakedFraction, alpha) : 0;
  const powShare = 1 - sAlpha;
  const posShare = sAlpha;

  // Referral comes from PoW share
  const referralPercent = powShare * REFERRAL_SHARE * 100;
  const miningPercent = powShare * (1 - REFERRAL_SHARE) * 100;
  const stakingPercent = posShare * 100;
  return {
    miningPercent,
    stakingPercent,
    referralPercent,
    powShare,
    posShare,
  };
}

export function simulate(params: SimParams): SimResult {
  const elapsedSeconds = params.elapsedDays * SECONDS_PER_DAY;

  const components = componentRates(elapsedSeconds);
  const totalRateAtomic = totalRateAtomicPerSecond(elapsedSeconds);
  const totalMintedAtomic = totalEmittedAtomic(elapsedSeconds);

  const emissionPerSecond = atomicToHuman(totalRateAtomic);

  const stakedFraction = Math.max(0, Math.min(1, params.stakedPercent / 100));
  const alpha = params.alpha;
  const beta = params.beta;

  const { miningPercent, stakingPercent, referralPercent, powShare } =
    computeSplit(stakedFraction, alpha);

  // Fee estimation
  const dailyFeeVolume = params.dailyTransfers * AVERAGE_TRANSFER_AMOUNT * FEE_PER_TRANSFER;
  const feeRatePerSecond = dailyFeeVolume / SECONDS_PER_DAY;

  // Gross rate G = emission + fees * (1 - beta)
  const grossRatePerSecond = emissionPerSecond + feeRatePerSecond * (1 - beta);

  // D_rate estimation: all network hashrate is computing at the same difficulty
  // D_rate = proofRate * avgDifficulty
  // proofRate = networkHashrate / 2^difficulty
  const hashesPerProof = params.difficultyBits > 0 ? 2 ** params.difficultyBits : 1;
  const networkProofRate = params.networkHashrate / hashesPerProof; // proofs/sec
  const dRate = networkProofRate * params.difficultyBits; // difficulty bits/sec

  // base_rate = (G * R_pow_share) / D_rate
  const baseRate = dRate > 0
    ? (grossRatePerSecond * powShare) / dRate
    : 0;

  // reward per proof = base_rate * d (minus referral cut for miner)
  const grossRewardPerProof = baseRate * params.difficultyBits;
  const rewardPerProof = grossRewardPerProof * (1 - REFERRAL_SHARE);

  // Per-split emission (per hour for readability)
  const emissionPerHour = emissionPerSecond * 3600;
  const miningEmissionPerHour = emissionPerHour * (miningPercent / 100);
  const stakingEmissionPerHour = emissionPerHour * (stakingPercent / 100);
  const referralEmissionPerHour = emissionPerHour * (referralPercent / 100);

  const totalMinted = atomicToHuman(totalMintedAtomic);
  const mintedPercent = (totalMintedAtomic / LI_TOTAL_SUPPLY_ATOMIC) * 100;

  const dailyFeeBurn = dailyFeeVolume * beta; // beta portion is burned
  const dailyEmission = emissionPerSecond * SECONDS_PER_DAY;
  const netDailyInflation = dailyEmission - dailyFeeBurn;

  // Gas cost per proof (uboot → LI conversion: approximate, price-dependent)
  // For display we show the raw uboot cost; if liPrice > 0 we can show USD equiv
  const gasCostPerProofLi = params.gasCostUboot / 1_000_000; // uboot to BOOT (6 decimals)
  const netRewardPerProof = Math.max(0, rewardPerProof - gasCostPerProofLi * params.liPrice);

  // Personal stats
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
    emissionPerHour,
    emissionPerSecond,
    miningPercent,
    stakingPercent,
    referralPercent,
    alpha,
    difficultyBits: params.difficultyBits,
    baseRate,
    rewardPerProof,
    totalMinted,
    mintedPercent,
    dailyFeeBurn,
    netDailyInflation,
    gasCostPerProofLi,
    netRewardPerProof,
    miningEmissionPerHour,
    stakingEmissionPerHour,
    referralEmissionPerHour,
    yourProofsPerDay,
    yourLiPerDay,
    yourLiPerMonth,
    timePerProofSeconds,
    yourNetworkSharePercent,
    yourDailyUsd,
    yourMonthlyUsd,
  };
}

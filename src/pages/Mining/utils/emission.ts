/**
 * 1:1 TypeScript port of litium-mine/src/emission.rs
 * All pure functions, zero dependencies.
 *
 * Validated: totalRateAtomicPerSecond(0) === 4_080_207_780_836_639
 */

const LI_TOTAL_SUPPLY_TOKENS = 1_000_000_000_000_000;
const LI_DECIMALS = 1_000_000;
const LI_TOTAL_SUPPLY_ATOMIC = LI_TOTAL_SUPPLY_TOKENS * LI_DECIMALS;
const COMPONENT_ALLOC_ATOMIC = LI_TOTAL_SUPPLY_ATOMIC / 7;

const MAIN_PHASE_SHARE_NUM = 0.9;
const TAIL_MONTHLY_SHARE = 0.01;
const MONTH_DAYS = 30.0;
const INFINITE_COMPONENT_YEARS = 20.0;
const SECONDS_PER_DAY = 86_400.0;

const FINITE_PERIODS = [1, 7, 30, 90, 365, 1461] as const;

type FiniteProfile = {
  alloc: number;
  lambda: number;
  tailK: number;
  crossoverSeconds: number;
  emittedAtCrossover: number;
  remainingAtCrossover: number;
};

function infiniteComponentDurationSeconds(): number {
  return INFINITE_COMPONENT_YEARS * 365.0 * SECONDS_PER_DAY;
}

function finiteProfile(periodDays: number): FiniteProfile {
  const alloc = COMPONENT_ALLOC_ATOMIC;
  const periodSeconds = periodDays * SECONDS_PER_DAY;
  const lambda = Math.log(10) / periodSeconds;
  const tailK = TAIL_MONTHLY_SHARE / (MONTH_DAYS * SECONDS_PER_DAY);
  const x = Math.max(0, Math.min(1, tailK / (9.0 * (lambda - tailK))));
  const crossoverSeconds = x > 0 ? -Math.log(x) / lambda : 0;
  const emittedAtCrossover =
    MAIN_PHASE_SHARE_NUM * alloc * (1 - Math.exp(-lambda * crossoverSeconds));
  const remainingAtCrossover = Math.max(0, alloc - emittedAtCrossover);

  return {
    alloc,
    lambda,
    tailK,
    crossoverSeconds,
    emittedAtCrossover,
    remainingAtCrossover,
  };
}

export function finiteComponentRate(
  elapsedSeconds: number,
  periodDays: number
): number {
  const p = finiteProfile(periodDays);
  if (elapsedSeconds <= p.crossoverSeconds) {
    return (
      MAIN_PHASE_SHARE_NUM *
      p.alloc *
      p.lambda *
      Math.exp(-p.lambda * elapsedSeconds)
    );
  }
  return (
    p.remainingAtCrossover *
    p.tailK *
    Math.exp(-p.tailK * (elapsedSeconds - p.crossoverSeconds))
  );
}

function finiteComponentEmitted(
  elapsedSeconds: number,
  periodDays: number
): number {
  const t = Math.max(0, elapsedSeconds);
  const p = finiteProfile(periodDays);
  if (t <= p.crossoverSeconds) {
    return MAIN_PHASE_SHARE_NUM * p.alloc * (1 - Math.exp(-p.lambda * t));
  }
  return (
    p.emittedAtCrossover +
    p.remainingAtCrossover *
      (1 - Math.exp(-p.tailK * (t - p.crossoverSeconds)))
  );
}

export function infiniteComponentRate(elapsedSeconds: number): number {
  if (elapsedSeconds >= infiniteComponentDurationSeconds()) {
    return 0;
  }
  return COMPONENT_ALLOC_ATOMIC / infiniteComponentDurationSeconds();
}

function infiniteComponentEmitted(elapsedSeconds: number): number {
  const duration = infiniteComponentDurationSeconds();
  const t = Math.max(0, Math.min(duration, elapsedSeconds));
  return COMPONENT_ALLOC_ATOMIC * (t / duration);
}

/** Total emission rate in atomic LI per second at the given elapsed time. */
export function totalRateAtomicPerSecond(elapsedSeconds: number): number {
  let sum = 0;
  for (const pd of FINITE_PERIODS) {
    sum += finiteComponentRate(elapsedSeconds, pd);
  }
  sum += infiniteComponentRate(elapsedSeconds);
  return sum;
}

/** Total cumulative emitted atomic LI at the given elapsed time. */
export function totalEmittedAtomic(elapsedSeconds: number): number {
  let sum = 0;
  for (const pd of FINITE_PERIODS) {
    sum += finiteComponentEmitted(elapsedSeconds, pd);
  }
  sum += infiniteComponentEmitted(elapsedSeconds);
  return sum;
}

export type ComponentRate = {
  name: string;
  periodDays: number;
  rate: number; // atomic LI/s
  color: string;
};

const COMPONENT_COLORS: Record<number, string> = {
  1: '#ef4444',
  7: '#f97316',
  30: '#eab308',
  90: '#22c55e',
  365: '#3b82f6',
  1461: '#a855f7',
  0: '#ec4899', // infinite
};

const COMPONENT_NAMES: Record<number, string> = {
  1: 'Li\u2081',
  7: 'Li\u2087',
  30: 'Li\u2083\u2080',
  90: 'Li\u2089\u2080',
  365: 'Li\u2083\u2086\u2085',
  1461: 'Li\u2081\u2084\u2086\u2081',
  0: 'Li\u221E',
};

/** Per-component rate breakdown at given elapsed time (7 components). */
export function componentRates(elapsedSeconds: number): ComponentRate[] {
  const rates: ComponentRate[] = FINITE_PERIODS.map((pd) => ({
    name: COMPONENT_NAMES[pd],
    periodDays: pd,
    rate: finiteComponentRate(elapsedSeconds, pd),
    color: COMPONENT_COLORS[pd],
  }));
  rates.push({
    name: COMPONENT_NAMES[0],
    periodDays: 0,
    rate: infiniteComponentRate(elapsedSeconds),
    color: COMPONENT_COLORS[0],
  });
  return rates;
}

/** Convert atomic LI to human LI. */
export function atomicToHuman(atomic: number): number {
  return atomic / LI_DECIMALS;
}

export { LI_TOTAL_SUPPLY_ATOMIC, LI_DECIMALS, FINITE_PERIODS, SECONDS_PER_DAY };

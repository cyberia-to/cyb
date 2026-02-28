import { useState, useMemo, useCallback, useRef } from 'react';
import { simulate, type SimParams } from '../utils/simulate';
import { compactLi } from '../utils/formatLi';
import EmissionChart from './EmissionChart';
import StatCard from './StatCard';
import styles from '../Mining.module.scss';

// ---------------------------------------------------------------------------
// Log-scale helpers
// ---------------------------------------------------------------------------

function logValue(linear01: number, min: number, max: number): number {
  const logMin = Math.log10(Math.max(1, min));
  const logMax = Math.log10(max);
  return Math.round(10 ** (logMin + linear01 * (logMax - logMin)));
}

function logToLinear(value: number, min: number, max: number): number {
  const logMin = Math.log10(Math.max(1, min));
  const logMax = Math.log10(max);
  const logVal = Math.log10(Math.max(1, value));
  return Math.max(0, Math.min(1, (logVal - logMin) / (logMax - logMin)));
}

// ---------------------------------------------------------------------------
// Formatters
// ---------------------------------------------------------------------------

function formatDays(days: number): string {
  if (days < 365) return `Day ${days}`;
  return `Year ${(days / 365).toFixed(1)}`;
}

function formatDuration(seconds: number): string {
  if (!isFinite(seconds)) return '\u221E';
  if (seconds < 60) return `${seconds.toFixed(1)}s`;
  if (seconds < 3600) return `${(seconds / 60).toFixed(1)}m`;
  if (seconds < 86400) return `${(seconds / 3600).toFixed(1)}h`;
  return `${(seconds / 86400).toFixed(1)}d`;
}

function compactUsd(val: number): string {
  if (val === 0) return '$0';
  const abs = Math.abs(val);
  const sign = val < 0 ? '-' : '';
  if (abs >= 1e12) return `${sign}$${(abs / 1e12).toFixed(2)}T`;
  if (abs >= 1e9) return `${sign}$${(abs / 1e9).toFixed(2)}B`;
  if (abs >= 1e6) return `${sign}$${(abs / 1e6).toFixed(2)}M`;
  if (abs >= 1e3) return `${sign}$${(abs / 1e3).toFixed(1)}K`;
  if (abs >= 1) return `${sign}$${abs.toFixed(2)}`;
  if (abs >= 0.01) return `${sign}$${abs.toFixed(4)}`;
  return `${sign}$${abs.toFixed(6)}`;
}

function compactNum(val: number): string {
  if (val === 0) return '0';
  const abs = Math.abs(val);
  if (abs >= 1e12) return `${(val / 1e12).toFixed(2)}T`;
  if (abs >= 1e9) return `${(val / 1e9).toFixed(2)}B`;
  if (abs >= 1e6) return `${(val / 1e6).toFixed(2)}M`;
  if (abs >= 1e3) return `${(val / 1e3).toFixed(1)}K`;
  if (abs >= 0.01) return val.toFixed(2);
  return val.toFixed(6);
}

// ---------------------------------------------------------------------------
// Slider+Input row component
// ---------------------------------------------------------------------------

type SliderInputProps = {
  label: string;
  value: number;
  onChange: (v: number) => void;
  min: number;
  max: number;
  step?: number;
  logScale?: boolean;
  suffix?: string;
};

function SliderInput({
  label,
  value,
  onChange,
  min,
  max,
  step,
  logScale,
  suffix,
}: SliderInputProps) {
  const [textValue, setTextValue] = useState(String(value));
  const [focused, setFocused] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const sliderValue = logScale ? logToLinear(value, min, max) : value;
  const sliderMin = logScale ? 0 : min;
  const sliderMax = logScale ? 1 : max;
  const sliderStep = logScale ? 0.002 : step ?? 1;

  const handleSlider = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const raw = Number(e.target.value);
      const actual = logScale ? logValue(raw, min, max) : raw;
      onChange(actual);
      setTextValue(String(actual));
    },
    [onChange, logScale, min, max]
  );

  const handleText = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      setTextValue(e.target.value);
    },
    []
  );

  const commitText = useCallback(() => {
    const parsed = Number(textValue);
    if (!isNaN(parsed)) {
      const clamped = Math.max(min, Math.min(max, parsed));
      onChange(clamped);
      setTextValue(String(clamped));
    } else {
      setTextValue(String(value));
    }
  }, [textValue, min, max, onChange, value]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter') commitText();
    },
    [commitText]
  );

  return (
    <div className={styles.simInputRow}>
      <div className={styles.simInputHeader}>
        <span className={styles.simInputLabel}>{label}</span>
        <div className={styles.simInputFieldWrap}>
          <input
            ref={inputRef}
            type="text"
            inputMode="numeric"
            data-sim-input={label}
            style={{
              width: 80,
              background: 'transparent',
              border: `1px solid ${focused ? '#36d6ae' : 'rgba(54, 214, 174, 0.25)'}`,
              borderRadius: 3,
              padding: '2px 6px',
              color: '#36d6ae',
              fontSize: 13,
              fontFamily: 'monospace',
              textAlign: 'right' as const,
              outline: 'none',
              boxShadow: 'none',
            }}
            value={focused ? textValue : String(value)}
            onChange={handleText}
            onFocus={() => { setFocused(true); setTextValue(String(value)); }}
            onBlur={() => { setFocused(false); commitText(); }}
            onKeyDown={handleKeyDown}
          />
          {suffix && (
            <span className={styles.simInputSuffix}>{suffix}</span>
          )}
        </div>
      </div>
      <input
        type="range"
        min={sliderMin}
        max={sliderMax}
        step={sliderStep}
        value={sliderValue}
        onChange={handleSlider}
        onPointerDown={(e) => e.stopPropagation()}
        onMouseDown={(e) => e.stopPropagation()}
        className={styles.simRange}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

type Props = {
  open: boolean;
};

function SimulatorSection({ open }: Props) {
  // Input states (direct values, not normalized)
  const [elapsedDays, setElapsedDays] = useState(1);
  const [networkHashrate, setNetworkHashrate] = useState(5000);
  const [yourHashrate, setYourHashrate] = useState(1000);
  const [stakedPct, setStakedPct] = useState(0);
  const [dailyTransfers, setDailyTransfers] = useState(100);
  const [difficultyBits, setDifficultyBits] = useState(15);
  const [targetSolutions, setTargetSolutions] = useState(100);
  const [epochSeconds, setEpochSeconds] = useState(600);
  const [liPrice, setLiPrice] = useState(0);

  const params: SimParams = useMemo(
    () => ({
      elapsedDays,
      networkHashrate,
      yourHashrate,
      stakedPercent: stakedPct,
      dailyTransfers,
      difficultyBits,
      targetSolutions,
      epochSeconds,
      liPrice,
    }),
    [
      elapsedDays,
      networkHashrate,
      yourHashrate,
      stakedPct,
      dailyTransfers,
      difficultyBits,
      targetSolutions,
      epochSeconds,
      liPrice,
    ]
  );

  const result = useMemo(() => simulate(params), [params]);

  if (!open) return null;

  return (
    <div className={styles.simulatorSection}>
      {/* Input sliders with text fields */}
      <div className={styles.simSliders}>
        <SliderInput
          label="Time since launch"
          value={elapsedDays}
          onChange={setElapsedDays}
          min={1}
          max={7300}
          logScale
          suffix={`(${formatDays(elapsedDays)})`}
        />
        <SliderInput
          label="Network hashrate"
          value={networkHashrate}
          onChange={setNetworkHashrate}
          min={100}
          max={10_000_000}
          logScale
          suffix="H/s"
        />
        <SliderInput
          label="Your hashrate"
          value={yourHashrate}
          onChange={setYourHashrate}
          min={1}
          max={10_000_000}
          logScale
          suffix="H/s"
        />
        <SliderInput
          label="Staked"
          value={stakedPct}
          onChange={setStakedPct}
          min={0}
          max={100}
          step={1}
          suffix="%"
        />
        <SliderInput
          label="Difficulty"
          value={difficultyBits}
          onChange={setDifficultyBits}
          min={0}
          max={64}
          step={1}
          suffix={`bits (eq: ${result.equilibriumDifficulty})`}
        />
        <SliderInput
          label="Target solutions"
          value={targetSolutions}
          onChange={setTargetSolutions}
          min={1}
          max={1000}
          logScale
          suffix="/epoch"
        />
        <SliderInput
          label="Epoch duration"
          value={epochSeconds}
          onChange={setEpochSeconds}
          min={60}
          max={3600}
          step={60}
          suffix="sec"
        />
        <SliderInput
          label="Daily transfers"
          value={dailyTransfers}
          onChange={setDailyTransfers}
          min={1}
          max={100_000}
          logScale
        />
        <SliderInput
          label="LI price"
          value={liPrice}
          onChange={setLiPrice}
          min={0}
          max={100}
          step={0.001}
          suffix="USD"
        />
      </div>

      {/* Emission chart */}
      <EmissionChart markerDays={elapsedDays} />

      {/* Legend */}
      <div className={styles.simLegend}>
        {result.components.map((c) => (
          <span key={c.name} className={styles.simLegendItem}>
            <span
              className={styles.simLegendDot}
              style={{ background: c.color }}
            />
            {c.name}
          </span>
        ))}
      </div>

      {/* Split bar */}
      <div className={styles.simSplitBar}>
        <div
          className={styles.simSplitSegment}
          style={{
            width: `${result.miningPercent}%`,
            background: '#36d6ae',
          }}
          title={`Mining ${result.miningPercent.toFixed(1)}%`}
        >
          {result.miningPercent > 15 &&
            `Mining ${result.miningPercent.toFixed(1)}%`}
        </div>
        <div
          className={styles.simSplitSegment}
          style={{
            width: `${result.stakingPercent}%`,
            background: '#3b82f6',
          }}
          title={`Staking ${result.stakingPercent.toFixed(1)}%`}
        >
          {result.stakingPercent > 10 &&
            `Staking ${result.stakingPercent.toFixed(1)}%`}
        </div>
        <div
          className={styles.simSplitSegment}
          style={{
            width: `${result.referralPercent}%`,
            background: '#a855f7',
          }}
          title={`Referral ${result.referralPercent.toFixed(1)}%`}
        >
          {result.referralPercent > 8 &&
            `Ref ${result.referralPercent.toFixed(0)}%`}
        </div>
      </div>

      {/* Network stats */}
      <span className={styles.sectionTitle}>Network</span>
      <div className={styles.statsGrid}>
        <StatCard
          label="Emission/epoch"
          value={compactLi(result.emissionPerEpoch)}
          suffix="LI"
        />
        <StatCard
          label="Mining/epoch"
          value={compactLi(result.miningEmissionPerEpoch)}
          suffix={`LI (${result.miningPercent.toFixed(1)}%)`}
        />
        <StatCard
          label="Staking/epoch"
          value={compactLi(result.stakingEmissionPerEpoch)}
          suffix={`LI (${result.stakingPercent.toFixed(1)}%)`}
        />
        <StatCard
          label="Referral/epoch"
          value={compactLi(result.referralEmissionPerEpoch)}
          suffix={`LI (${result.referralPercent.toFixed(0)}%)`}
        />
        <StatCard
          label="Reward/proof"
          value={compactLi(result.rewardPerProof)}
          suffix="LI"
        />
        <StatCard
          label="Minted"
          value={`${result.mintedPercent.toFixed(2)}%`}
        />
        <StatCard
          label="Alpha"
          value={`${(result.alpha * 100).toFixed(0)}%`}
        />
        <StatCard
          label="Net inflation/day"
          value={compactLi(result.netDailyInflation)}
          suffix="LI"
        />
        <StatCard
          label="Daily burn"
          value={compactLi(result.dailyBurn)}
          suffix="LI"
        />
      </div>

      {/* Personal stats */}
      <span className={styles.sectionTitle}>Your miner</span>
      <div className={styles.statsGrid}>
        <StatCard
          label="Proofs/day"
          value={compactNum(result.yourProofsPerDay)}
        />
        <StatCard
          label="Time/proof"
          value={formatDuration(result.timePerProofSeconds)}
        />
        <StatCard
          label="LI/day"
          value={compactLi(result.yourLiPerDay)}
          suffix="LI"
        />
        <StatCard
          label="LI/month"
          value={compactLi(result.yourLiPerMonth)}
          suffix="LI"
        />
        <StatCard
          label="Network share"
          value={`${Math.min(result.yourNetworkSharePercent, 9999).toFixed(2)}%`}
        />
        {liPrice > 0 && (
          <StatCard label="USD/day" value={compactUsd(result.yourDailyUsd)} />
        )}
        {liPrice > 0 && (
          <StatCard
            label="USD/month"
            value={compactUsd(result.yourMonthlyUsd)}
          />
        )}
      </div>
    </div>
  );
}

export default SimulatorSection;

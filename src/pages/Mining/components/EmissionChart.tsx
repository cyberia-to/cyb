import { useMemo } from 'react';
import {
  componentRates,
  SECONDS_PER_DAY,
  type ComponentRate,
} from '../utils/emission';

type Props = {
  /** Current slider position in days */
  markerDays: number;
  /** Chart width */
  width?: number;
  /** Chart height */
  height?: number;
};

const MAX_DAYS = 7300; // 20 years
const SAMPLES = 200;

/**
 * SVG stacked area chart showing 7 emission components over 20 years.
 * Vertical marker line at the current slider position.
 */
function EmissionChart({ markerDays, width = 600, height = 200 }: Props) {
  // Precompute sample points for all 7 components
  const { paths, markerX, labels } = useMemo(() => {
    const step = MAX_DAYS / SAMPLES;
    const sampleDays: number[] = [];
    for (let i = 0; i <= SAMPLES; i++) {
      sampleDays.push(i * step);
    }

    // Get rates for each sample point
    const allRates: ComponentRate[][] = sampleDays.map((d) =>
      componentRates(d * SECONDS_PER_DAY)
    );

    const numComponents = allRates[0].length;

    // Find max stacked rate for Y normalization
    let maxTotal = 0;
    const totals = allRates.map((rates) => {
      const total = rates.reduce((s, r) => s + r.rate, 0);
      if (total > maxTotal) maxTotal = total;
      return total;
    });
    if (maxTotal === 0) maxTotal = 1;

    const pad = 4;
    const chartW = width - pad * 2;
    const chartH = height - pad * 2;

    // Build stacked area paths (bottom-up)
    const xCoords = sampleDays.map((d) => pad + (d / MAX_DAYS) * chartW);
    const stackedPaths: { d: string; color: string; name: string }[] = [];

    // Cumulative baselines per sample point
    const baselines = new Array(sampleDays.length).fill(0);

    for (let c = numComponents - 1; c >= 0; c--) {
      const topY: number[] = [];
      const bottomY: number[] = [];

      for (let i = 0; i < sampleDays.length; i++) {
        const rate = allRates[i][c].rate;
        const base = baselines[i];
        const top = base + rate;
        baselines[i] = top;

        // Map to SVG Y (inverted)
        topY.push(pad + chartH - (top / maxTotal) * chartH);
        bottomY.push(pad + chartH - (base / maxTotal) * chartH);
      }

      // Build path: top line forward, bottom line backward
      let d = `M${xCoords[0]},${topY[0]}`;
      for (let i = 1; i < sampleDays.length; i++) {
        d += `L${xCoords[i]},${topY[i]}`;
      }
      for (let i = sampleDays.length - 1; i >= 0; i--) {
        d += `L${xCoords[i]},${bottomY[i]}`;
      }
      d += 'Z';

      stackedPaths.push({
        d,
        color: allRates[0][c].color,
        name: allRates[0][c].name,
      });
    }

    // Marker X position
    const mx = pad + (Math.min(markerDays, MAX_DAYS) / MAX_DAYS) * chartW;

    // X-axis labels
    const axisLabels = [0, 1, 2, 5, 10, 20].map((yr) => ({
      text: yr === 0 ? '0' : `${yr}yr`,
      x: pad + ((yr * 365) / MAX_DAYS) * chartW,
    }));

    return { paths: stackedPaths, markerX: mx, labels: axisLabels };
  }, [markerDays, width, height]);

  return (
    <svg
      width="100%"
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      preserveAspectRatio="xMidYMid meet"
      style={{ display: 'block' }}
    >
      {paths.map((p) => (
        <path key={p.name} d={p.d} fill={p.color} opacity={0.7} />
      ))}

      {/* Vertical marker */}
      <line
        x1={markerX}
        y1={4}
        x2={markerX}
        y2={height - 16}
        stroke="#fff"
        strokeWidth={1.5}
        strokeDasharray="4,3"
        opacity={0.8}
      />

      {/* X-axis labels */}
      {labels.map((l) => (
        <text
          key={l.text}
          x={l.x}
          y={height - 2}
          fill="#666"
          fontSize={10}
          textAnchor="middle"
        >
          {l.text}
        </text>
      ))}
    </svg>
  );
}

export default EmissionChart;

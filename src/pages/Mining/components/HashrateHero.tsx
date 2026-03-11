import cx from 'classnames';
import styles from '../Mining.module.scss';

type Props = {
  hashrate: number;
  isActive: boolean;
  samples: number[];
  sessionAvg?: number;
};

function buildSparklinePath(samples: number[], width: number, height: number) {
  if (samples.length < 2) return { line: '', fill: '' };

  const max = Math.max(...samples, 1);
  const step = width / (samples.length - 1);

  const points = samples.map((s, i) => ({
    x: i * step,
    y: height - (s / max) * height * 0.85 - height * 0.05,
  }));

  const line = points.map((p, i) => `${i === 0 ? 'M' : 'L'}${p.x},${p.y}`).join(' ');
  const fill = `${line} L${points[points.length - 1].x},${height} L${points[0].x},${height} Z`;

  return { line, fill };
}

function HashrateHero({ hashrate, isActive, samples, sessionAvg }: Props) {
  const { line, fill } = buildSparklinePath(samples, 300, 60);

  return (
    <div className={styles.heroContainer}>
      <div className={cx(styles.heroValue, { [styles.heroPulse]: isActive })}>
        {hashrate.toFixed(0)}
        <span className={styles.heroUnit}> H/s</span>
      </div>

      <div className={styles.heroAvg} style={{ visibility: isActive && sessionAvg != null && sessionAvg > 0 ? 'visible' : 'hidden' }}>
        avg {sessionAvg != null && sessionAvg > 0 ? sessionAvg.toFixed(0) : '0'} H/s
      </div>

      <svg className={styles.sparkline} viewBox="0 0 300 60" preserveAspectRatio="none">
        {samples.length >= 2 && (
          <>
            <defs>
              <linearGradient id="sparkFill" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="#36d6ae" stopOpacity="0.3" />
                <stop offset="100%" stopColor="#36d6ae" stopOpacity="0" />
              </linearGradient>
            </defs>
            <path d={fill} fill="url(#sparkFill)" />
            <path d={line} fill="none" stroke="#36d6ae" strokeWidth="2" />
          </>
        )}
      </svg>
    </div>
  );
}

export default HashrateHero;

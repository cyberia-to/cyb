import { useCallback, useEffect, useRef, useState } from 'react';
import { useAppData } from 'src/contexts/appData';
import styles from './Electricity.module.scss';

const R = (min, max) => Math.round(min + Math.random() * (max - min));

const f = (p, P, d) => [(p[0] - P[0]) * d + P[0], (p[1] - P[1]) * d + P[1]];

function generatePath() {
  const L = 2050;
  const C = R(9, 10);
  const PC = L / C;
  const A = [];
  const D = 10;
  const RF = 0.4;
  const yPos = 15;
  let NP = 'M';

  for (let i = 0; i < C; i += 1) {
    if (i === 0) {
      A.push([i, yPos]);
    } else if (i < C / 2) {
      A.push([i * PC, R(-D, D) * i]);
    } else {
      A.push([i * PC, R(-D, D) * (C - i)]);
    }
  }

  for (let i = 0; i < C; i += 1) {
    if (i !== 0 && i !== C - 1) {
      const P = f(A[i - 1], A[i], RF);
      const p = f(A[i], A[i + 1], 1 - RF);
      NP += ` L${P[0]},${P[1]}`;
      NP += ` Q${A[i][0]},${A[i][1]}`;
      NP += ` ${p[0]},${p[1]}`;
    } else if (i === C - 1) {
      NP += ` T${L},${yPos}`;
    } else {
      NP += ` ${A[i][0]},${A[i][1]}`;
    }
  }

  return { path: NP, strokeBase: R(-2, 5) };
}

function Electricity() {
  const [data, setData] = useState('M0,0 L240,0');
  const [strokeBase, setStrokeBase] = useState(1);
  const [stage, setStage] = useState(false);
  const { block } = useAppData();
  const prevBlock = useRef(null);
  const animating = useRef(false);

  const strike = useCallback(() => {
    if (animating.current) return;
    animating.current = true;

    let frames = 0;
    const totalFrames = 18; // ~600ms at 30fps

    setStage(true);

    const timerId = setInterval(() => {
      const { path, strokeBase: sb } = generatePath();
      setData(path);
      setStrokeBase(sb);
      frames += 1;

      if (frames >= totalFrames) {
        clearInterval(timerId);
        setStage(false);
        animating.current = false;
      }
    }, 1000 / 30);
  }, []);

  useEffect(() => {
    if (block && block !== prevBlock.current) {
      prevBlock.current = block;
      strike();
    }
  }, [block, strike]);

  return (
    <div className={styles.electricity}>
      <div className={styles.line}>
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 2050 80">
          <g id="lightningContainer">
            <rect className={styles.electricityLineRect} width="2050" height="80" />
            {stage && (
              <g width="2050" height="80" transform="translate(0, 40)" opacity="1">
                <path
                  stroke="rgba(0,238,255,0.1)"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={strokeBase + 12}
                  fill="none"
                  d={data}
                />
                <path
                  stroke="rgba(0,238,255,0.3)"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={strokeBase + 6}
                  fill="none"
                  d={data}
                />
                <path
                  stroke="#fff"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={strokeBase}
                  fill="none"
                  d={data}
                />
              </g>
            )}
          </g>
        </svg>
      </div>
    </div>
  );
}

export default Electricity;

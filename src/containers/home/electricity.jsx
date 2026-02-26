import { useEffect, useRef, useState } from 'react';
import { useAppData } from 'src/contexts/appData';
import styles from './Electricity.module.scss';

const M = Math;

function Electricity() {
  const [data, setData] = useState('M0,0 L240,0');
  const [stage, setStage] = useState(false);
  const { block } = useAppData();
  const prevBlockRef = useRef(block);

  const At = (el, a, v) => {
    try {
      if (el !== null) {
        el.setAttribute(a, v);
      }
    } catch (error) {
      console.error(error);
    }
  };

  const R = (min, max) => {
    return M.round(min + M.random() * (max - min));
  };

  const f = (p, P, d) => {
    return [(p[0] - P[0]) * d + P[0], (p[1] - P[1]) * d + P[1]];
  };

  useEffect(() => {
    if (!block || block === prevBlockRef.current) {
      return;
    }
    prevBlockRef.current = block;

    const DOC = document;
    const l0 = DOC.getElementById('lightning0');
    const l1 = DOC.getElementById('lightning1');
    const l2 = DOC.getElementById('lightning2');

    setStage(true);

    const timerId = setInterval(() => {
      const L = 2050;
      const C = R(9, 10);
      const PC = L / C;
      const A = [];
      const D = 10;
      let NP = 'M';
      const B = R(-2, 5);
      const RF = 0.4;
      const yPos = 15;

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
      At(l0, 'stroke-width', B + 12);
      At(l1, 'stroke-width', B + 6);
      At(l2, 'stroke-width', B);
      setData(NP);
    }, 1000 / 30);

    const stopId = setTimeout(() => {
      clearInterval(timerId);
      setStage(false);
    }, 600);

    return () => {
      clearInterval(timerId);
      clearTimeout(stopId);
    };
  }, [block]);

  return (
    <div className={styles.electricity}>
      <div className={styles.line}>
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 2050 80">
          <g id="lightningContainer">
            <rect className={styles.electricityLineRect} width="2050" height="80" />
            {stage && (
              <g id="lightningG" width="2050" height="80" transform="translate(0, 40)" opacity="1">
                <path
                  id="lightning0"
                  stroke="rgba(0,238,255,0.1)"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth="12"
                  fill="none"
                  d={data}
                />
                <path
                  id="lightning1"
                  stroke="rgba(0,238,255,0.3)"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth="5"
                  fill="none"
                  d={data}
                />
                <path
                  id="lightning2"
                  stroke="#fff"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth="1"
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

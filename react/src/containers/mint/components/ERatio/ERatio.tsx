import { Tooltip } from 'src/components';
import QuestionBtn from 'src/components/Rank/QuestionBtn/QuestionBtn';
import styles from './ERatio.module.scss';

function ERatio({ eRatio }: { eRatio: number }) {
  return (
    <div>
      <div className={styles.containerText}>
        E-Ratio
        <div className={styles.icon}>
          <Tooltip
            placement="top"
            tooltip="Efficiency ratio show how much of your energy is not utilzed. Investmint H into A or V to improve your E-Ratio. The lower E-Ratio the more benefit you miss"
          >
            <QuestionBtn />
          </Tooltip>
        </div>
      </div>
      <div className={styles.svgWrap}>
        <svg viewBox="0 0 40 40">
          <circle className={styles.svgBackground} />
          <circle
            className={styles.svgCalc}
            strokeDasharray={`${eRatio * 1.27} 127`}
            strokeLinecap="round"
            strokeWidth="8"
          />
        </svg>
        <div className={styles.svgValue}>{eRatio}</div>
      </div>
    </div>
  );
}

export default ERatio;

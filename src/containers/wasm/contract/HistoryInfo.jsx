import ReactJson from 'react-json-view';
import { Link } from 'react-router-dom';
import { v4 as uuidv4 } from 'uuid';
import { CardCantainer } from '../ui/ui';
import styles from './stylesHistoryInfo.scss';

function CodeLink({ codeId, text }) {
  return <Link to={`/libs/${codeId}`}>{text || `Lib #${codeId}`}</Link>;
}

function HistoryInfo({ contractCodeHistory }) {
  return (
    <CardCantainer>
      <div className={styles.containerHistoryInfo}>
        <span className={styles.containerHistoryInfoTitle}>History</span>
        {contractCodeHistory.map((item) => {
          const key = uuidv4();

          return (
            <div key={key} className={styles.containerHistoryInfoItem}>
              <div className={styles.containerHistoryInfoItemTitle}>
                {item.operation}-<CodeLink codeId={item.codeId} />
              </div>
              <ReactJson
                src={item.msg}
                theme="twilight"
                displayObjectSize={false}
                displayDataTypes={false}
              />
            </div>
          );
        })}
      </div>
    </CardCantainer>
  );
}

export default HistoryInfo;

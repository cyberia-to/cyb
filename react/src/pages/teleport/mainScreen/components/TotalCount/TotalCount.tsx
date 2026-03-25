import { Link } from 'react-router-dom';
import styles from './TotalCount.module.scss';

type Props = {
  value: number | string;
  text?: string;
  to: string;
};

function TotalCount({ value, text, to }: Props) {
  return (
    <Link to={to} className={styles.containerTotalCount}>
      <div className={styles.countValue}>+{value}</div>
      {text && <span className={styles.countText}>{text}</span>}
    </Link>
  );
}

export default TotalCount;

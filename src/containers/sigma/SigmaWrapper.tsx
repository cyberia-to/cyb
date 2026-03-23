import Sigma from '.';
import styles from './SigmaWrapper.module.scss';

function SigmaWrapper() {
  return (
    <div className={styles.wrapper}>
      <Sigma />
    </div>
  );
}

export default SigmaWrapper;

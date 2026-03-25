import statusTrueImg from 'src/image/ionicons_svg_ios-checkmark-circle.svg';
import statusFalseImg from 'src/image/ionicons_svg_ios-close-circle.svg';
import styles from './TableDataTxs.module.scss';

function StatusTxs({ success }: { success: boolean }) {
  return (
    <img className={styles.imgStatus} src={success ? statusTrueImg : statusFalseImg} alt="status" />
  );
}

export default StatusTxs;

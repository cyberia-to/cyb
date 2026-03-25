import styles from '../styles.scss';

function RowItem({ children }) {
  return <div className={styles.containerRowItem}>{children}</div>;
}

export default RowItem;

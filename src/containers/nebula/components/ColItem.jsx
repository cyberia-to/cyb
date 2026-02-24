import styles from '../styles.scss';

function ColItem({ children, justifyContent = 'flex-start', ...props }) {
  return (
    <div style={{ justifyContent }} className={styles.containerColItem} {...props}>
      {children}
    </div>
  );
}

export default ColItem;

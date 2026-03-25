import styles from './TeleportText.module.scss';

function TeleportText() {
  return (
    <div className={styles.container}>
      <span className={styles.title}>communicate</span>

      <span className={styles.description}>by exchanging value</span>
    </div>
  );
}

export default TeleportText;

import AccountCount from 'src/containers/brain/accountCount';
import Txs from 'src/containers/brain/tx';
import styles from './TeleportStat.module.scss';

function Value({ children }: { children: React.ReactNode }) {
  return <span className={styles.value}>{children}</span>;
}

function TeleportStat() {
  return (
    <span className={styles.container}>
      <Value>
        <Txs />
      </Value>
      transactions submitted <br />
      by
      <Value>
        <AccountCount />
      </Value>
      neurons
    </span>
  );
}

export default TeleportStat;

import styles from './LogRelayer.module.scss';

function MessageItem({ text = '' }) {
  return (
    <div className={styles.containerMessageItem}>
      <span>{text}</span>
    </div>
  );
}

function LogRelayer({ relayerLog }) {
  return (
    <div className={styles.containerLogRelayer}>
      {relayerLog &&
        Object.keys(relayerLog).length > 0 &&
        relayerLog.map((item, i) => <MessageItem text={item} key={i} />)}
    </div>
  );
}

export default LogRelayer;

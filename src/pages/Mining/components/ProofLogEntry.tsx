import { Link } from 'react-router-dom';
import { routes } from 'src/routes';
import { trimString } from 'src/utils/utils';
import Pill from 'src/components/Pill/Pill';
import styles from '../Mining.module.scss';

type Props = {
  index: number;
  hash: string;
  txHash?: string;
  error?: string;
  timestamp: number;
};

function timeAgo(timestamp: number): string {
  const diff = Math.floor((Date.now() - timestamp) / 1000);
  if (diff < 60) return `${diff}s ago`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  return `${Math.floor(diff / 3600)}h ago`;
}

function ProofLogEntry({ index, hash, txHash, error, timestamp }: Props) {
  return (
    <div className={styles.proofEntry}>
      <span className={styles.proofEntryIndex}>#{index}</span>
      <span className={styles.proofEntryHash}>{trimString(hash, 8, 4)}</span>
      <span className={styles.proofEntryStatus}>
        {txHash ? (
          <Link to={routes.txExplorer.getLink(txHash)}>
            {trimString(txHash, 8, 4)} <Pill color="green" text="OK" />
          </Link>
        ) : error ? (
          <span title={error}>
            {error.slice(0, 30)} <Pill color="red" text="FAIL" />
          </span>
        ) : null}
      </span>
      <span className={styles.proofEntryTime}>{timeAgo(timestamp)}</span>
    </div>
  );
}

export default ProofLogEntry;

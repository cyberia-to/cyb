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
  status?: 'submitted' | 'pending' | 'success' | 'failed';
  timestamp: number;
};

function timeAgo(timestamp: number): string {
  const diff = Math.floor((Date.now() - timestamp) / 1000);
  if (diff < 60) return `${diff}s ago`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  return `${Math.floor(diff / 3600)}h ago`;
}

function StatusPill({ status, error }: { status?: string; error?: string }) {
  switch (status) {
    case 'success':
      return <Pill color="green" text="OK" />;
    case 'failed':
      return <Pill color="red" text="FAIL" />;
    case 'pending':
      return <Pill color="yellow" text="PENDING" />;
    case 'submitted':
      return <Pill color="yellow" text="SENT" />;
    default:
      // Legacy entries without status field
      if (error) return <Pill color="red" text="FAIL" />;
      return <Pill color="green" text="OK" />;
  }
}

function ProofLogEntry({ index, hash, txHash, error, status, timestamp }: Props) {
  return (
    <div className={styles.proofEntry}>
      <span className={styles.proofEntryIndex}>#{index}</span>
      <span className={styles.proofEntryHash}>{trimString(hash, 8, 4)}</span>
      <span className={styles.proofEntryStatus}>
        {txHash ? (
          <Link to={routes.txExplorer.getLink(txHash)}>
            {trimString(txHash, 8, 4)} <StatusPill status={status} error={error} />
          </Link>
        ) : (
          <span title={error || ''}>
            {error ? error.slice(0, 30) : ''} <StatusPill status={status} error={error} />
          </span>
        )}
      </span>
      <span className={styles.proofEntryTime}>{timeAgo(timestamp)}</span>
    </div>
  );
}

export default ProofLogEntry;

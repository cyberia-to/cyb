import { Link } from 'react-router-dom';
import { useAppSelector } from 'src/redux/hooks';
import { routes } from 'src/routes';
import styles from './MiningBadge.module.scss';

function formatHashrate(hps: number): string {
  if (hps >= 1_000_000) return `${(hps / 1_000_000).toFixed(1)} MH/s`;
  if (hps >= 1_000) return `${(hps / 1_000).toFixed(1)} KH/s`;
  return `${hps.toFixed(0)} H/s`;
}

function MiningBadge() {
  const active = useAppSelector((state) => state.mining.active);
  const hashrate = useAppSelector((state) => state.mining.hashrate);

  if (!active) {
    return null;
  }

  return (
    <Link to={routes.mining.path} className={styles.badge}>
      <span className={styles.dot} />
      {formatHashrate(hashrate)}
    </Link>
  );
}

export default MiningBadge;

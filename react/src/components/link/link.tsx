import { Link } from 'react-router-dom';
import { trimString } from 'src/utils/utils';
import styles from './cid.modules.scss';

export function LinkWindow({ to, children, ...props }) {
  return (
    <a target="_blank" {...props} rel="noopener noreferrer" href={to}>
      {children}
    </a>
  );
}

type Props = {
  cid: string;
  children?: React.ReactNode;
};

export function Cid({ cid, children }: Props) {
  return (
    <Link to={`/ipfs/${cid}`} className={styles.cid}>
      {children || trimString(cid, 5, 4)}
    </Link>
  );
}

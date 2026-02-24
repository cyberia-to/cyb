import { Link } from 'react-router-dom';
import { trimString } from '../../../../utils/utils';
import { CardItem } from '../code/index';
import styles from './styles.scss';

function CodeInfo({ uploadTxHash, details }) {
  const { creator, checksum } = details;
  return (
    <div className={styles.containerCodeDetailsCodeInfo}>
      <CardItem
        title="Upload tx"
        value={
          <Link to={`/network/bostrom/tx/${uploadTxHash}`}>{trimString(uploadTxHash, 6, 6)}</Link>
        }
      />
      <CardItem
        title="Creator"
        value={<Link to={`/network/bostrom/contract/${creator}`}>{trimString(creator, 10)}</Link>}
      />
      <CardItem title="Checksum" value={trimString(checksum, 8, 8)} />
    </div>
  );
}

export default CodeInfo;

import { ValidatorTableData } from 'src/pages/Sphere/types/tableData';
import { Link } from 'react-router-dom';
import { routes } from 'src/routes';
import KeybaseAvatar from 'src/pages/Sphere/pages/components/KeybaseAvatar/keybaseAvatar';
import styles from './Moniker.module.scss';
import { StatusTooltip } from '../../../ui';

function Moniker({
  data,
  status,
  operatorAddress,
}: {
  data: ValidatorTableData['description'];
  status: ValidatorTableData['status'];
  operatorAddress: ValidatorTableData['operatorAddress'];
}) {
  const { moniker, identity } = data;
  return (
    <div className={styles.monikerContainer}>
      <StatusTooltip status={status} />
      <KeybaseAvatar identity={identity} />
      <Link
        to={routes.hero.getLink(operatorAddress)}
        className={styles.moniker}
      >
        {moniker}
      </Link>
    </div>
  );
}

export default Moniker;

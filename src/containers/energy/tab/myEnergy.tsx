import { Pane } from '@cybercongress/gravity';
import { Link } from 'react-router-dom';
import { routes } from 'src/routes';
import { DenomArr, Dots } from '../../../components';
import { formatNumber } from '../../../utils/utils';
import { Card, TableSlots } from '../ui';
import styles from './myEnergy.module.scss';

// TODO: finish
type Props = {
  balancesResource: {
    milliampere: number | null;
    millivolt: number | null;
  };
  slotsData: unknown;
  loadingAuthAccounts: unknown;
};

function MyEnergy({ slotsData, balancesResource, loadingAuthAccounts }: Props) {
  return (
    <div>
      <div
        style={{
          padding: '15px',
        }}
      >
        <Pane marginY={30} textAlign="center">
          <Link to={routes.oracle.ask.getLink('energy')}>Energy </Link> (W) is the product of{' '}
          <Link to={routes.oracle.ask.getLink('amper')}>ampers </Link> and{' '}
          <Link to={routes.oracle.ask.getLink('volt')}>volts</Link>
        </Pane>
        <Pane marginBottom={20} fontSize="20px">
          Balance:
        </Pane>
        <div className={styles.balanceRow}>
          <Card
            title={<DenomArr denomValue="milliampere" />}
            value={balancesResource.milliampere ? formatNumber(balancesResource.milliampere) : 0}
            stylesContainer={{ maxWidth: '200px' }}
          />
          <span className={styles.operator}>x</span>
          <Card
            title={<DenomArr denomValue="millivolt" />}
            value={balancesResource.millivolt ? formatNumber(balancesResource.millivolt) : 0}
            stylesContainer={{ maxWidth: '200px' }}
          />
          <span className={styles.operator}>=</span>
          <Card
            title="W"
            value={
              balancesResource.millivolt && balancesResource.milliampere
                ? formatNumber(balancesResource.millivolt * balancesResource.milliampere)
                : 0
            }
            stylesContainer={{ maxWidth: '200px' }}
          />
        </div>
      </div>

      {loadingAuthAccounts ? <Dots big /> : <TableSlots data={slotsData} />}
    </div>
  );
}

export default MyEnergy;

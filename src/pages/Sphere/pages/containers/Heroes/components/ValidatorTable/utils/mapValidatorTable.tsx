import { createColumnHelper } from '@tanstack/react-table';
import BigNumber from 'bignumber.js';
import { FormatNumber } from 'src/components';
import IconsNumber from 'src/components/IconsNumber/IconsNumber';
import cx from 'classnames';
import { ValidatorTableData } from '../../../../../../types/tableData';
import Moniker from '../ui/Moniker/Moniker';
import VotingPower from '../ui/VotingPower/VoitingPower';
import styles from './mapValidatorTable.module.scss';

const columnHelper = createColumnHelper<ValidatorTableData>();

const renderColumnsData = () => [
  columnHelper.accessor('id', {
    header: 'id',
    size: 70,
    cell: (info) => {
      const { id, rank } = info.row.original;
      return <span className={cx(styles.idStatus, styles[rank])}>{id}</span>;
    },
  }),
  columnHelper.accessor('description.moniker', {
    header: 'moniker',
    enableSorting: false,
    cell: (info) => {
      const { status } = info.row.original;
      const { operatorAddress } = info.row.original;
      return (
        <Moniker
          data={info.cell.row.original.description}
          status={status}
          operatorAddress={operatorAddress}
        />
      );
    },
  }),
  columnHelper.accessor('tokens', {
    header: 'Power',
    sortingFn: (rowA, rowB) => {
      const a = parseFloat(rowA.original.tokens);
      const b = parseFloat(rowB.original.tokens);
      return a - b;
    },
    cell: (info) => {
      return <VotingPower data={info.row.original} />;
    },
  }),
  columnHelper.accessor('apr', {
    header: 'APR',
    size: 80,
    sortingFn: (rowA, rowB) => {
      const a = rowA.original.apr;
      const b = rowB.original.apr;
      return a - b;
    },
    cell: (info) => {
      const val = info.getValue();
      return (
        <FormatNumber
          currency="%"
          number={new BigNumber(val)
            .multipliedBy(100)
            .dp(2, BigNumber.ROUND_FLOOR)
            .toFixed(2)}
        />
      );
    },
    aggregationFn: 'mean',
    aggregatedCell: ({ getValue }) => {
      return new BigNumber(getValue())
        .multipliedBy(100)
        .dp(2, BigNumber.ROUND_FLOOR)
        .toFixed(2);
    },
  }),
  columnHelper.accessor('delegation', {
    header: 'Your bond',
    sortingFn: (rowA, rowB) => {
      const a = rowA.original.delegation?.amount
        ? parseFloat(rowA.original.delegation.amount)
        : 0;
      const b = rowB.original.delegation?.amount
        ? parseFloat(rowB.original.delegation.amount)
        : 0;
      return a - b;
    },
    cell: (info) => {
      const item = info.getValue();
      return item ? (
        <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
          <IconsNumber value={item.amount} type={item.denom} />
        </div>
      ) : (
        ''
      );
    },
  }),
];

export default renderColumnsData;

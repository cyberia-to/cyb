import { useTransactionCountQuery } from 'src/generated/graphql';
import { Dots } from '../../components';
import { formatNumber } from '../../utils/utils';

function Txs() {
  const { loading, data, error } = useTransactionCountQuery();

  if (loading) {
    return <Dots />;
  }

  if (error) {
    return <span>∞</span>;
  }

  return <span>{formatNumber(data.transaction_aggregate.aggregate.count)}</span>;
}

export default Txs;

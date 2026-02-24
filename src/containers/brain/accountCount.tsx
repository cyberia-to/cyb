import { useAccountCountQuery } from 'src/generated/graphql';
import { Dots } from '../../components';
import { formatNumber } from '../../utils/utils';

function AccountCount() {
  const { loading, data, error } = useAccountCountQuery();
  if (loading) {
    return <Dots />;
  }
  if (error) {
    return <span>∞</span>;
  }

  return <span>{formatNumber(data?.account_aggregate.aggregate?.count || 0)}</span>;
}

export default AccountCount;

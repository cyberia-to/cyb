import BigNumber from 'bignumber.js';
import { Dots } from 'src/components';
import { INFINITY } from 'src/constants/app';
import { useUptimeByAddressQuery } from 'src/generated/graphql';
import { consensusPubkey } from 'src/utils/utils';

function Uptime({ consensusPub }: { consensusPub: string }) {
  const { loading, data, error } = useUptimeByAddressQuery({
    variables: {
      address: `${consensusPubkey(consensusPub)}`,
    },
  });

  if (loading) {
    return <Dots />;
  }

  if (error) {
    return INFINITY;
  }

  return `${new BigNumber(data?.uptime[0].uptime || 0)
    .shiftedBy(2)
    .dp(2, BigNumber.ROUND_FLOOR)
    .toString()} %`;
}

export default Uptime;

import { useUptimeByAddressQuery } from 'src/generated/graphql';
import { INFINITY } from 'src/constants/app';
import BigNumber from 'bignumber.js';
import { consensusPubkey } from 'src/utils/utils';
import { Dots } from 'src/components';

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

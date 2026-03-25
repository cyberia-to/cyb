import BigNumber from 'bignumber.js';
import { useMemo } from 'react';
import { Link } from 'react-router-dom';
import { Time } from 'src/components';
import { getNowUtcNumber } from 'src/utils/date';
import useGetTimeCreatePassport from './api/api';

type Props = {
  address: string;
};

function FirstTx({ address }: Props) {
  const data = useGetTimeCreatePassport(address);

  const time = useMemo(() => {
    if (!data) {
      return undefined;
    }

    const timestamp = data.txResponses[0]?.timestamp as string | undefined;

    if (!timestamp) {
      return undefined;
    }

    return new BigNumber(getNowUtcNumber()).minus(Date.parse(timestamp)).toNumber();
  }, [data]);

  if (!time) {
    return null;
  }

  return (
    <Link to="./time">
      <Time msTime={time} />
    </Link>
  );
}

export default FirstTx;

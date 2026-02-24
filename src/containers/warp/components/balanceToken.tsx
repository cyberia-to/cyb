import { Pane } from '@cybercongress/gravity';
import { useMemo } from 'react';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { ObjKeyValue } from 'src/types/data';
import { Dots } from '../../../components';
import { formatNumber, getDisplayAmount } from '../../../utils/utils';

type BalanceTokenProps = {
  token: string;
  data: ObjKeyValue | null;
};

function BalanceToken({ token, data }: BalanceTokenProps) {
  const { tracesDenom } = useIbcDenom();

  const balance = useMemo(() => {
    if (token.length > 0 && data?.[token]) {
      const [{ coinDecimals }] = tracesDenom(token);
      return getDisplayAmount(data[token], coinDecimals);
    }
    return 0;
  }, [data, token, tracesDenom]);

  return (
    <Pane
      display="flex"
      alignItems="center"
      color="#777777"
      fontSize="18px"
      width="100%"
      justifyContent="space-between"
      marginBottom={12}
    >
      <Pane>Available</Pane>
      <Pane>{!data ? <Dots /> : formatNumber(balance)}</Pane>
    </Pane>
  );
}

export default BalanceToken;

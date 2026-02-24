import { CSSProperties } from 'react';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { convertAmount } from 'src/utils/utils';
import FormatNumberTokens from '../FormatNumberTokens/FormatNumberTokens';

type Props = {
  amountValue: number;
  denom: string;
  styleValue?: CSSProperties;
};

function AmountDenom({ amountValue, denom, styleValue }: Props) {
  const { tracesDenom } = useIbcDenom();

  let amount = 0;

  if (amountValue && amountValue > 0) {
    const [{ coinDecimals }] = tracesDenom(denom);
    amount = convertAmount(amountValue, coinDecimals);
  }

  return <FormatNumberTokens text={denom} value={amount} styleValue={styleValue} />;
}

export default AmountDenom;

import { Pane } from '@cybercongress/gravity';
import { BASE_DENOM } from 'src/constants/config';
import { formatNumber } from '../../utils/utils';
import DenomArr from '../denom';

function NumberCurrency({
  amount,
  fontSizeDecimal,
  currencyNetwork = BASE_DENOM,
  onlyImg,
  onlyText = true,
  ...props
}) {
  // const number = formatNumber(amount / CYBER.DIVISOR_CYBER_G, 3);
  return (
    <Pane display="grid" gridTemplateColumns="1fr 45px" gridGap="5px" {...props}>
      <Pane whiteSpace="nowrap" display="flex" alignItems="center">
        <span>{formatNumber(Math.floor(amount))}</span>
      </Pane>
      <div style={{ textAlign: 'start', alignItems: 'center', display: 'flex' }}>
        <DenomArr denomValue={currencyNetwork} onlyText={onlyText} onlyImg={onlyImg} />
      </div>
    </Pane>
  );
}

export default NumberCurrency;

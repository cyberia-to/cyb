import { Pane } from '@cybercongress/gravity';
import { CardStatisics, Vitalik } from '../../../components';
import { formatCurrency } from '../../../utils/utils';

function LiquidityParam({ data }) {
  // TODO add pool_types
  try {
    if (!data) {
      return null;
    }
    return (
      <Pane
        display="grid"
        gridTemplateColumns="repeat(auto-fit, minmax(250px, 1fr))"
        gridGap="20px"
      >
        <CardStatisics
          title="pool creation fee"
          value={formatCurrency(
            parseFloat(data.pool_creation_fee[0].amount),
            data.pool_creation_fee[0].denom
          )}
        />
        <CardStatisics
          title="swap fee rate"
          value={`${parseFloat(data.swap_fee_rate) * 100} %`}
        />
        <CardStatisics
          title="withdraw fee rate"
          value={`${parseFloat(data.withdraw_fee_rate) * 100} %`}
        />
        <CardStatisics
          title="max order amount ratio"
          value={`${parseFloat(data.max_order_amount_ratio) * 100} %`}
        />
        <CardStatisics
          title="min init deposit amount"
          value={data.min_init_deposit_amount}
        />
        <CardStatisics
          title="init pool coin mint amount"
          value={data.init_pool_coin_mint_amount}
        />
        <CardStatisics
          title="max reserve coin amount"
          value={data.max_reserve_coin_amount}
        />
        <CardStatisics
          title="unit batch height"
          value={data.unit_batch_height}
        />
        <CardStatisics
          title="circuit_breaker_enabled"
          value={`${data.circuit_breaker_enabled}`}
        />
      </Pane>
    );
  } catch (error) {
    console.warn('LiquidityParam', error);
    return (
      <Pane
        justifyContent="center"
        flexDirection="column"
        alignItems="center"
        display="flex"
      >
        <Vitalik />
        Error !
      </Pane>
    );
  }
}

export default LiquidityParam;

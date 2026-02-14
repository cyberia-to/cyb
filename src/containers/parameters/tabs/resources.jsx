/* eslint-disable radix */
import { Pane } from '@cybercongress/gravity';
import { CardStatisics, Vitalik } from '../../../components';
import { formatCurrency, dhm } from '../../../utils/utils';

function ResourcesParam({ data }) {
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
        <CardStatisics title="max slots" value={data.max_slots} />
        <CardStatisics
          title="base investmint period ampere"
          value={`${dhm(parseInt(data.base_investmint_period_ampere * 1000))}`}
        />
        <CardStatisics
          title="base investmint period volt"
          value={`${dhm(parseInt(data.base_investmint_period_volt * 1000))}`}
        />
        <CardStatisics
          title="halving period ampere blocks"
          value={data.halving_period_ampere_blocks}
        />
        <CardStatisics
          title="halving period volt blocks"
          value={data.halving_period_volt_blocks}
        />
        <CardStatisics
          title="min investmint period"
          value={`${dhm(parseInt(data.min_investmint_period * 1000))}`}
        />
        <CardStatisics
          title="base investmint amount ampere"
          value={formatCurrency(
            parseFloat(data.base_investmint_amount_ampere.amount),
            data.base_investmint_amount_ampere.denom
          )}
        />
        <CardStatisics
          title="base investmint amount volt"
          value={formatCurrency(
            parseFloat(data.base_investmint_amount_volt.amount),
            data.base_investmint_amount_volt.denom
          )}
        />
      </Pane>
    );
  } catch (error) {
    console.warn('ResourcesParam', error);
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

export default ResourcesParam;

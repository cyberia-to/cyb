import { Pane } from '@cybercongress/gravity';
import { CardStatisics, Vitalik } from '../../../components';
import { dhm } from '../../../utils/utils';

function StakingParam({ data }) {
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
          title="unbonding time"
          value={dhm(parseFloat(data.unbonding_time) * 1000)}
        />
        <CardStatisics
          title="max validators"
          value={parseFloat(data.max_validators)}
        />
        <CardStatisics
          title="max entries"
          value={parseFloat(data.max_entries)}
        />
        <CardStatisics
          title="historical entries"
          value={parseFloat(data.historical_entries)}
        />
      </Pane>
    );
  } catch (error) {
    console.warn('StakingParam', error);
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

export default StakingParam;

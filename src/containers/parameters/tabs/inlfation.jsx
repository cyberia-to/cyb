import { Pane } from '@cybercongress/gravity';
import { CardStatisics, Vitalik } from '../../../components';

function InlfationParam({ data }) {
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
          title="inflation rate change"
          value={`${parseFloat(data.inflation_rate_change) * 100} %`}
        />
        <CardStatisics
          title="inflation max"
          value={`${parseFloat(data.inflation_max) * 100} %`}
        />
        <CardStatisics
          title="inflation min"
          value={`${parseFloat(data.inflation_min) * 100} %`}
        />
        <CardStatisics
          title="goal bonded"
          value={`${parseFloat(data.goal_bonded) * 100} %`}
        />
      </Pane>
    );
  } catch (error) {
    console.warn('BandwidthParam', error);
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

export default InlfationParam;

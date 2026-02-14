import { Pane } from '@cybercongress/gravity';
import { CardStatisics, Vitalik } from '../../../components';

function DistributionParam({ data }) {
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
          title="community tax"
          value={`${parseFloat(data.community_tax) * 100} %`}
        />
        <CardStatisics
          title="base proposer reward"
          value={`${parseFloat(data.base_proposer_reward) * 100} %`}
        />
        <CardStatisics
          title="bonus proposer reward"
          value={`${parseFloat(data.bonus_proposer_reward) * 100} %`}
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

export default DistributionParam;

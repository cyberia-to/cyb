import { Pane } from '@cybercongress/gravity';
import { CardStatisics, Vitalik } from '../../../components';
import { dhm } from '../../../utils/utils';

function SlashingParam({ data }) {
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
          title="signed blocks window"
          value={parseFloat(data.signed_blocks_window)}
        />
        <CardStatisics
          title="min signed per window"
          value={`${parseFloat(data.min_signed_per_window) * 100} %`}
        />
        <CardStatisics
          title="downtime jail duration"
          value={`${dhm(parseFloat(data.downtime_jail_duration) * 1000)}`}
        />
        <CardStatisics
          title="slash fraction double sign"
          value={`${parseFloat(data.slash_fraction_double_sign) * 100} %`}
        />
        <CardStatisics
          title="slash fraction downtime"
          value={`${parseFloat(data.slash_fraction_downtime) * 100} %`}
        />
      </Pane>
    );
  } catch (error) {
    console.warn('SlashingParam', error);
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

export default SlashingParam;

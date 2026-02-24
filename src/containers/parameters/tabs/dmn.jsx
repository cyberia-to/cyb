import { Pane } from '@cybercongress/gravity';
import { CardStatisics, Vitalik } from '../../../components';

function DmnParam({ data }) {
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
        <CardStatisics title="max gas" value={data.max_gas} />
        <CardStatisics title="fee ttl" value={data.fee_ttl} />
      </Pane>
    );
  } catch (error) {
    console.warn('DmnParam', error);
    return (
      <Pane justifyContent="center" flexDirection="column" alignItems="center" display="flex">
        <Vitalik />
        Error !
      </Pane>
    );
  }
}

export default DmnParam;

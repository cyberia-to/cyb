import { Pane } from '@cybercongress/gravity';
import { CardStatisics, Vitalik } from '../../../components';

function RankParam({ data }) {
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
          title="calculation period"
          value={parseFloat(data.calculation_period)}
        />
        <CardStatisics
          title="damping factor"
          value={parseFloat(data.damping_factor)}
        />
        <CardStatisics title="tolerance" value={parseFloat(data.tolerance)} />
      </Pane>
    );
  } catch (error) {
    console.warn('RankParam', error);
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

export default RankParam;

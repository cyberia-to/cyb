import { Pane } from '@cybercongress/gravity';
import { CardStatisics, Vitalik } from '../../../components';

function GridParam({ data }) {
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
        <CardStatisics title="max routes" value={data.max_routes} />
      </Pane>
    );
  } catch (error) {
    console.warn('ParamGrid', error);
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

export default GridParam;

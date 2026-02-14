import { Pane } from '@cybercongress/gravity';
import { CardStatisics, Vitalik } from '../../../components';
import { formatNumber } from '../../../utils/utils';

function BandwidthParam({ data }) {
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
          title="base price"
          value={formatNumber(parseFloat(data.base_price))}
        />
        <CardStatisics
          title="recovery period blocks"
          value={formatNumber(parseFloat(data.recovery_period))}
        />
        <CardStatisics
          title="adjust price period blocks"
          value={formatNumber(parseFloat(data.adjust_price_period))}
        />
        <CardStatisics
          title="max block bandwidth"
          value={formatNumber(parseFloat(data.max_block_bandwidth))}
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

export default BandwidthParam;

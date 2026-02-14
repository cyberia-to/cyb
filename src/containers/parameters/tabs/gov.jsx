import { Pane } from '@cybercongress/gravity';
import { CardStatisics, Vitalik } from '../../../components';
import { formatCurrency, dhm } from '../../../utils/utils';

// Parse duration string like "604800s" to milliseconds for dhm()
function parseDurationToMs(duration) {
  return parseFloat(duration) * 1000;
}

function GovParam({ data }) {
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
          title="quorum"
          value={`${parseFloat(data.tally.quorum) * 100} %`}
        />
        <CardStatisics
          title="threshold"
          value={`${parseFloat(data.tally.threshold) * 100} %`}
        />
        <CardStatisics
          title="veto"
          value={`${parseFloat(data.tally.veto_threshold) * 100} %`}
        />
        <CardStatisics
          title="min deposit"
          value={
            data.deposit.min_deposit?.length > 0
              ? formatCurrency(
                  parseFloat(data.deposit.min_deposit[0].amount),
                  data.deposit.min_deposit[0].denom
                )
              : '—'
          }
        />
        <CardStatisics
          title="max deposit period"
          value={dhm(parseDurationToMs(data.deposit.max_deposit_period))}
        />
        <CardStatisics
          title="voting period"
          value={dhm(parseDurationToMs(data.voting.voting_period))}
        />
      </Pane>
    );
  } catch (error) {
    console.warn('GovParam', error);
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

export default GovParam;

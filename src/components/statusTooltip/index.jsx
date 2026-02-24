import { Pane, Pill, Text, Tooltip } from '@cybercongress/gravity';

// const statusHeroes = {
//   BOND_STATUS_UNSPECIFIED: 0,
//   /** BOND_STATUS_UNBONDED - UNBONDED defines a validator that is not bonded. */
//   BOND_STATUS_UNBONDED: 1,
//   /** BOND_STATUS_UNBONDING - UNBONDING defines a validator that is unbonding. */
//   BOND_STATUS_UNBONDING: 2,
//   /** BOND_STATUS_BONDED - BONDED defines a validator that is bonded. */
//   BOND_STATUS_BONDED: 3,
// };

function StatusTooltip({ status, size }) {
  let statusColor;
  switch (status) {
    case 1:
      statusColor = 'red';
      break;
    case 2:
      statusColor = 'yellow';
      break;
    case 3:
      statusColor = 'green';
      break;
    default:
      statusColor = 'neutral';
      break;
  }

  return (
    <Pane display="flex" alignItems="center">
      <Tooltip
        appearance="card"
        content={
          <Pane display="flex" alignItems="center" paddingX={18} paddingY={18}>
            <Text>
              Validator status:&nbsp;
              {status === 1 && 'unbonded'}
              {status === 2 && 'unbonding'}
              {status === 3 && 'bonded'}
            </Text>
          </Pane>
        }
      >
        <Pill
          height={size || 7}
          width={size || 7}
          borderRadius="50%"
          paddingX={4}
          paddingY={0}
          // marginX={20}
          isSolid
          color={statusColor}
        />
      </Tooltip>
    </Pane>
  );
}

export default StatusTooltip;

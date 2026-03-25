import { fromBase64, fromUtf8 } from '@cosmjs/encoding';
import ReactJson from 'react-json-view';
import { Link } from 'react-router-dom';
import { Account, AmountDenom, Cid, DenomArr } from 'src/components';
import { BASE_DENOM } from 'src/constants/config';
import Row from '../../components/Row/Row';
import { formatNumber } from '../../utils/search/utils';
import { timeSince } from '../../utils/utils';
import ContainerMsgsType from './components/ContainerMsgsType';

const S_TO_MS = 1 * 10 ** 3;

function MultiSend({ msg }) {
  return (
    <ContainerMsgsType type={msg['@type']}>
      <Row
        title="Inputs"
        value={msg.inputs.map((data, i) => {
          return (
            <div
              key={i}
              style={{
                marginBottom: '5px',
              }}
            >
              <Account address={data.address}>
                {data.coins.map((coin, j) => {
                  return (
                    <span key={j}>
                      {' '}
                      <AmountDenom amountValue={coin.amount} denom={coin.denom} />
                    </span>
                  );
                })}
              </Account>
            </div>
          );
        })}
      />
      <Row
        title="Outputs"
        value={msg.outputs.map((data, i) => {
          return (
            <div
              key={i}
              style={{
                marginBottom: '5px',
              }}
            >
              <Account address={data.address} />
              {data.coins.map((coin, j) => {
                return (
                  <span key={j}>
                    {' '}
                    <AmountDenom amountValue={coin.amount} denom={coin.denom} />
                  </span>
                );
              })}
            </div>
          );
        })}
      />
    </ContainerMsgsType>
  );
}

function MsgLink({ msg }) {
  return (
    <ContainerMsgsType type={msg['@type']}>
      <Row title="Neuron" value={<Account address={msg.neuron} />} />
      {msg.links.map((item, index) => (
        <div
          key={`${item.from}-${index}`}
          style={{
            padding: '10px',
            paddingBottom: 0,
            marginBottom: '10px',
            borderTop: '1px solid #3ab79366',
          }}
        >
          <Row title="from" value={<Cid cid={item.from} />} />
          <Row title="to" value={<Cid cid={item.to} />} />
        </div>
      ))}
    </ContainerMsgsType>
  );
}

function MsgInvestmint({ msg }) {
  return (
    <ContainerMsgsType type={msg['@type']}>
      {msg.neuron && <Row title="neuron" value={<Account address={msg.neuron} />} />}
      {msg.amount && (
        <Row
          title="amount"
          value={<AmountDenom amountValue={msg.amount.amount} denom={msg.amount.denom} />}
        />
      )}
      {msg.resource && (
        <Row title="resource" value={<DenomArr gap={8} denomValue={msg.resource} />} />
      )}
      {msg.length && <Row title="length" value={timeSince(msg.length * S_TO_MS)} />}
    </ContainerMsgsType>
  );
}

function MsgCreateRoute({ msg }) {
  return (
    <ContainerMsgsType type={msg['@type']}>
      <Row title="source" value={<Account address={msg.source} />} />
      <Row title="name" value={msg.name} />
      <Row title="destination" value={<Account address={msg.destination} />} />
    </ContainerMsgsType>
  );
}

function MsgEditRoute({ msg }) {
  return (
    <ContainerMsgsType type={msg['@type']}>
      <Row title="source" value={<Account address={msg.source} />} />
      {msg && (
        <Row title="amount" value={<AmountDenom amountValue={msg.amount} denom={msg.denom} />} />
      )}
      <Row title="destination" value={<Account address={msg.destination} />} />
    </ContainerMsgsType>
  );
}

function MsgDeleteRoute({ msg }) {
  return (
    <ContainerMsgsType type={msg['@type']}>
      <Row title="source" value={<Account address={msg.source} />} />
      <Row title="destination" value={<Account address={msg.destination} />} />
    </ContainerMsgsType>
  );
}

function MsgEditRouteName({ msg }) {
  return <MsgCreateRoute msg={msg} />;
}

function Activites({ msg }) {
  let type = '';

  if (msg['@type']) {
    type = msg['@type'];
  }

  if (type.includes('MsgCyberlink')) {
    return <MsgLink msg={msg} />;
  }

  // bank
  if (type.includes('MsgSend')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="From" value={<Account address={msg.from_address} />} />
        <Row title="To" value={<Account address={msg.to_address} />} />
        <Row
          title="amount"
          value={
            msg.amount.length > 0
              ? msg.amount.map((amount, i) => {
                  const { denom, amount: amountValue } = amount;
                  if (i > 0) {
                    return (
                      <>
                        {' '}
                        ,<AmountDenom amountValue={amountValue} denom={denom} />
                      </>
                    );
                  }
                  return <AmountDenom amountValue={amountValue} denom={denom} key={i} />;
                })
              : `0 ${BASE_DENOM.toUpperCase()}`
          }
        />
      </ContainerMsgsType>
    );
  }

  // case 'cosmos-sdk/MsgMultiSend':
  if (type.includes('MsgMultiSend')) {
    return <MultiSend msg={msg} />;
  }

  // staking
  if (type.includes('MsgCreateValidator')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="Delegator" value={<Account address={msg.delegator_address} />} />
        <Row title="Validator" value={<Account address={msg.validator_address} />} />
        <Row
          title="Mininum Self Delegation"
          value={`${formatNumber(msg.min_self_delegation)}
           ${BASE_DENOM.toUpperCase()}`}
        />
        <Row title="Commission Rate" value={msg.commission.rate} />
        <Row title="Description" value={msg.description.moniker} />
      </ContainerMsgsType>
    );
  }
  if (type.includes('MsgEditValidator')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="Address" value={<Account address={msg.address} />} />
      </ContainerMsgsType>
    );
  }
  if (type.includes('MsgDelegate')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="Delegator" value={<Account address={msg.delegator_address} />} />
        <Row title="Validator" value={<Account address={msg.validator_address} />} />
        <Row
          title="Delegation Amount"
          value={<AmountDenom amountValue={msg.amount.amount} denom={msg.amount.denom} />}
        />
      </ContainerMsgsType>
    );
  }
  if (type.includes('MsgUndelegate')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="Delegator" value={<Account address={msg.delegator_address} />} />
        <Row title="Validator" value={<Account address={msg.validator_address} />} />
        <Row
          title="Undelegation Amount"
          value={<AmountDenom amountValue={msg.amount.amount} denom={msg.amount.denom} />}
        />
      </ContainerMsgsType>
    );
  }
  if (type.includes('MsgBeginRedelegate')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="Delegator" value={<Account address={msg.delegator_address} />} />
        <Row title="Redelegated from" value={<Account address={msg.validator_src_address} />} />
        <Row title="Redelegated to" value={<Account address={msg.validator_dst_address} />} />
        <Row
          title="Redelegation Amount"
          value={<AmountDenom amountValue={msg.amount.amount} denom={msg.amount.denom} />}
        />
      </ContainerMsgsType>
    );
  }

  // swap
  if (type.includes('MsgSwapWithinBatch')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row
          title="Swap requester address"
          value={<Account address={msg.swap_requester_address} />}
        />
        <Row
          title="Demand coin denom"
          value={<DenomArr gap={8} denomValue={msg.demand_coin_denom} />}
        />
        <Row
          title="Offer coin"
          value={<AmountDenom amountValue={msg.offer_coin.amount} denom={msg.offer_coin.denom} />}
        />
        <Row
          title="Offer coin fee"
          value={
            <AmountDenom amountValue={msg.offer_coin_fee.amount} denom={msg.offer_coin_fee.denom} />
          }
        />
        <Row title="Order price" value={msg.order_price} />
        <Row title="Pool ID" value={msg.pool_id} />
        <Row title="Swap type id" value={msg.swap_type_id} />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgDepositWithinBatch')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="Depositor address" value={<Account address={msg.depositor_address} />} />
        <Row title="Pool id" value={msg.pool_id} />
        <Row
          title="Deposit coins"
          value={
            <div style={{ display: 'flex', gap: '10px', flexDirection: 'column' }}>
              {msg.deposit_coins.map((data, i) => {
                return <AmountDenom amountValue={data.amount} key={i} denom={data.denom} />;
              })}
            </div>
          }
        />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgWithdrawWithinBatch')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="Withdrawer address" value={<Account address={msg.withdrawer_address} />} />
        <Row title="Pool id" value={msg.pool_id} />
        <Row
          title="Pool coin"
          value={<AmountDenom amountValue={msg.pool_coin.amount} denom={msg.pool_coin.denom} />}
        />
      </ContainerMsgsType>
    );
  }

  // distribution
  if (type.includes('MsgWithdrawValidatorCommission')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="address" value={<Account address={msg.validator_address} />} />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgWithdrawDelegatorReward')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="delegator" value={<Account address={msg.delegator_address} />} />
        <Row title="validator" value={<Account address={msg.validator_address} />} />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgModifyWithdrawAddress')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="delegator" value={<Account address={msg.delegator_address} />} />
      </ContainerMsgsType>
    );
  }

  // gov
  if (type.includes('MsgSubmitProposal')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="Proposer" value={<Account address={msg.proposer} />} />
        <Row
          title="Initail Deposit"
          value={
            msg.initial_deposit.length > 0
              ? msg.initial_deposit.map((amount, i) => {
                  if (i > 0) {
                    return ` ,${amount.amount} ${amount.denom}`;
                  }
                  return `${formatNumber(amount.amount)} ${amount.denom.toUpperCase()}`;
                })
              : `0 ${BASE_DENOM.toUpperCase()}`
          }
        />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgDeposit')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row
          title="Proposal Id"
          value={<Link to={`/senate/${msg.proposal_id}`}>{msg.proposal_id}</Link>}
        />
        <Row title="Depositor" value={<Account address={msg.depositor} />} />
        <Row
          title="Amount"
          value={msg.amount.map((amount, i) => {
            if (i > 0) {
              return ` ,${amount.amount} ${amount.denom}`;
            }
            return `${formatNumber(amount.amount)} ${amount.denom.toUpperCase()}`;
          })}
        />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgVote')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row
          title="Proposal Id"
          value={<Link to={`/senate/${msg.proposal_id}`}>{msg.proposal_id}</Link>}
        />
        <Row title="Voter" value={<Account address={msg.voter} />} />
        <Row title="Vote Option" value={msg.option} />
      </ContainerMsgsType>
    );
  }

  // slashing
  if (type.includes('MsgUnjail')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="address" value={<Account address={msg.validator_addr || msg.address} />} />
      </ContainerMsgsType>
    );
  }

  // wasm
  if (type.includes('MsgInstantiateContract')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="address" value={<Account address={msg.sender} />} />
        <Row title="label" value={msg.label} />
        {msg.code_id && <Row title="code id" value={msg.code_id} />}
        <Row
          title="message"
          value={
            <ReactJson
              src={msg.msg}
              theme="twilight"
              displayObjectSize={false}
              displayDataTypes={false}
            />
          }
        />
        <Row
          title="funds"
          value={
            msg.funds.length > 0
              ? msg.funds.map((amount, i) => {
                  if (i > 0) {
                    return ` ,${amount.amount} ${amount.denom}`;
                  }
                  return `${formatNumber(amount.amount)} ${amount.denom.toUpperCase()}`;
                })
              : `0 ${BASE_DENOM.toUpperCase()}`
          }
        />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgStoreCode')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="address" value={<Account address={msg.sender} />} />
        <Row title="wasm byte code" value={msg.wasm_byte_code} />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgExecuteContract')) {
    console.log(msg.msg);

    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="address" value={<Account address={msg.sender} />} />
        <Row title="contract" value={<Account address={msg.contract} />} />
        <Row
          title="message"
          value={
            <ReactJson
              src={msg.msg}
              theme="twilight"
              displayObjectSize={false}
              displayDataTypes={false}
            />
          }
        />
        <Row
          title="funds"
          value={
            msg.funds.length > 0
              ? msg.funds.map((amount, i) => {
                  if (i > 0) {
                    return ` ,${amount.amount} ${amount.denom}`;
                  }
                  return `${formatNumber(amount.amount)} ${amount.denom.toUpperCase()}`;
                })
              : `0 ${BASE_DENOM.toUpperCase()}`
          }
        />
      </ContainerMsgsType>
    );
  }

  // Investmint
  if (type.includes('MsgInvestmint')) {
    return <MsgInvestmint msg={msg} />;
  }

  // grid
  if (type.includes('MsgCreateRoute')) {
    return <MsgCreateRoute msg={msg} />;
  }

  if (type.includes('MsgEditRoute')) {
    return <MsgEditRoute msg={msg} />;
  }

  if (type.includes('MsgDeleteRoute')) {
    return <MsgDeleteRoute msg={msg} />;
  }

  if (type.includes('MsgEditRouteName')) {
    return <MsgEditRouteName msg={msg} />;
  }

  if (type.includes('MsgTransfer')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="Sender" value={<Account address={msg.sender} />} />
        <Row title="Receiver" value={<Account address={msg.receiver} />} />
        <Row title="Source channel" value={msg.source_channel} />
        <Row title="Source port" value={msg.source_port} />
        <Row
          title="Token"
          value={<AmountDenom amountValue={msg.token.amount} denom={msg.token.denom} />}
        />
        <Row
          title="Timeout height"
          value={`${formatNumber(
            msg.timeout_height.revision_height
          )} / ${formatNumber(msg.timeout_height.revision_number)}`}
        />
      </ContainerMsgsType>
    );
  }

  // ibc
  if (type.includes('MsgRecvPacket')) {
    const { packet } = msg;
    if (Object.hasOwn(packet, 'data')) {
      try {
        const dataPacketSring = fromUtf8(fromBase64(packet.data));
        packet.data = dataPacketSring;
      } catch (_e) {
        // That string wasn't valid.
      }
    }
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="Signer" value={<Account address={msg.signer} />} />
        <Row
          title="Packet"
          value={
            <ReactJson
              src={msg.packet}
              theme="twilight"
              displayObjectSize={false}
              displayDataTypes={false}
            />
          }
        />
        <Row title="Proof Commitment" value={msg.proof_commitment} />
        <Row
          title="Proof Height"
          value={
            <ReactJson
              src={msg.proof_height}
              theme="twilight"
              displayObjectSize={false}
              displayDataTypes={false}
            />
          }
        />
        {/* <Row
          title="Proof Height"
          value={`RN: ${formatNumber(
            msg.proof_height.revision_number
          )} RH: ${formatNumber(msg.proof_height.revision_height)}`}
        /> */}
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgUpdateClient')) {
    return (
      <ContainerMsgsType type={msg['@type']}>
        <Row title="Signer" value={<Account address={msg.signer} />} />
        <Row title="Client ID" value={msg.client_id} />
        <Row
          title="Header"
          value={
            <ReactJson
              src={msg.header}
              theme="twilight"
              displayObjectSize={false}
              displayDataTypes={false}
            />
          }
        />
      </ContainerMsgsType>
    );
  }

  return (
    <div>
      <ReactJson src={msg} theme="twilight" displayObjectSize={false} displayDataTypes={false} />
    </div>
  );
}

export default Activites;

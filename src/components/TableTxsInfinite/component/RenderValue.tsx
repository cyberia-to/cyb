import { ReactNode } from 'react';
import { Link } from 'react-router-dom';
import { Account, AmountDenom, Cid, DenomArr, FormatNumberTokens } from 'src/components';
import { formatNumber, timeSince, trimString } from 'src/utils/utils';

const S_TO_MS = 1 * 10 ** 3;

function ContainerMsgsType({ children, alignItems }: { children: ReactNode; alignItems?: string }) {
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: alignItems || 'flex-start',
        gap: '5px',
        width: '100%',
      }}
    >
      {children}
    </div>
  );
}

function Row({ title, value }) {
  return (
    <div
      style={{
        display: 'flex',
        gap: '5px',
        width: '100%',
        justifyContent: 'space-between',
      }}
    >
      <div>{title}:</div>
      <div>{value}</div>
    </div>
  );
}

function MsgCreateRoute({ value }) {
  return (
    <ContainerMsgsType>
      <Row title="source" value={<Account address={value.source} />} />
      <Row title="name" value={value.name} />
      <Row title="destination" value={<Account address={value.destination} />} />
    </ContainerMsgsType>
  );
}

function MsgEditRoute({ value }) {
  return (
    <ContainerMsgsType>
      <Row title="source" value={<Account address={value.source} />} />
      {value && (
        <Row
          title="amount"
          value={<AmountDenom amountValue={value.amount} denom={value.denom} />}
        />
      )}
      <Row title="destination" value={<Account address={value.destination} />} />
    </ContainerMsgsType>
  );
}

function MsgDeleteRoute({ value }) {
  return (
    <ContainerMsgsType>
      <Row title="source" value={<Account address={value.source} />} />
      <Row title="destination" value={<Account address={value.destination} />} />
    </ContainerMsgsType>
  );
}

function RenderValue({ value, type, accountUser }) {
  if (type.includes('MsgCyberlink')) {
    return (
      <ContainerMsgsType>
        <Row title="neuron" value={<Account address={value.neuron} />} />
        {value.links.length === 1 &&
          value.links.map((item) => (
            <>
              <Row title="from" value={<Cid cid={item.from}>{trimString(item.from, 6, 6)}</Cid>} />
              <Row title="to" value={<Cid cid={item.to}>{trimString(item.to, 6, 6)}</Cid>} />
            </>
          ))}
        {value.links.length > 1 && <Row title="links" value={formatNumber(value.links.length)} />}
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgSend')) {
    return (
      <ContainerMsgsType>
        <Row title="from" value={<Account address={value.from_address} />} />
        <Row title="to" value={<Account address={value.to_address} />} />
        <Row
          title="amount"
          value={
            <ContainerMsgsType>
              {value.amount.map((item, i) => {
                return <AmountDenom denom={item.denom} amountValue={item.amount} key={i} />;
              })}
            </ContainerMsgsType>
          }
        />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgMultiSend')) {
    return (
      <ContainerMsgsType>
        {value.inputs.map((itemInputs) => (
          <>
            <Row title="from" value={<Account address={itemInputs.address} />} />
            <Row
              title="inputs coins"
              value={
                <ContainerMsgsType>
                  {itemInputs.coins.map((itemCoins, i) => {
                    return (
                      <AmountDenom denom={itemCoins.denom} amountValue={itemCoins.amount} key={i} />
                    );
                  })}
                </ContainerMsgsType>
              }
            />
          </>
        ))}
        {value.outputs.map((itemOutputs) => {
          if (itemOutputs.address === accountUser) {
            return (
              <>
                <Row title="to" value={<Account address={itemOutputs.address} />} />
                <Row
                  title="outputs coins"
                  value={
                    <ContainerMsgsType>
                      {itemOutputs.coins.map((itemCoins, i) => {
                        return (
                          <AmountDenom
                            key={i}
                            denom={itemCoins.denom}
                            amountValue={itemCoins.amount}
                          />
                        );
                      })}
                    </ContainerMsgsType>
                  }
                />
              </>
            );
          }
          return null;
        })}
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgInvestmint')) {
    return (
      <ContainerMsgsType>
        <Row title="neuron" value={<Account address={value.neuron} />} />
        <Row
          title="amount"
          value={<AmountDenom denom={value.amount.denom} amountValue={value.amount.amount} />}
        />
        <Row title="resource" value={<DenomArr gap={8} denomValue={value.resource} />} />
        <Row title="length" value={timeSince(value.length * S_TO_MS)} />
      </ContainerMsgsType>
    );
  }

  // swap
  if (type.includes('MsgSwapWithinBatch')) {
    return (
      <ContainerMsgsType>
        <Row title="requester" value={<Account address={value.swap_requester_address} />} />
        <Row title="demand" value={<DenomArr gap={8} denomValue={value.demand_coin_denom} />} />
        <Row
          title="offer coin"
          value={
            <AmountDenom amountValue={value.offer_coin.amount} denom={value.offer_coin.denom} />
          }
        />
        <Row title="Order price" value={<FormatNumberTokens value={value.order_price} />} />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgDepositWithinBatch')) {
    return (
      <ContainerMsgsType>
        <Row title="from" value={<Account address={value.depositor_address} />} />
        <Row title="pool id" value={value.pool_id} />
        <Row
          title="deposit"
          value={
            <ContainerMsgsType alignItems="flex-end">
              {value.deposit_coins.map((data, i) => {
                return <AmountDenom amountValue={data.amount} denom={data.denom} key={i} />;
              })}
            </ContainerMsgsType>
          }
        />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgWithdrawWithinBatch')) {
    return (
      <ContainerMsgsType>
        <Row title="withdrawer" value={<Account address={value.withdrawer_address} />} />
        <Row title="pool id" value={value.pool_id} />
        <Row
          title="pool coin"
          value={<AmountDenom amountValue={value.pool_coin.amount} denom={value.pool_coin.denom} />}
        />
      </ContainerMsgsType>
    );
  }

  // staking - distribution
  if (type.includes('MsgDelegate')) {
    return (
      <ContainerMsgsType>
        <Row title="delegator" value={<Account address={value.delegator_address} />} />
        <Row title="validator" value={<Account address={value.validator_address} />} />
        <Row
          title="amount"
          value={<AmountDenom amountValue={value.amount.amount} denom={value.amount.denom} />}
        />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgCreateValidator')) {
    return (
      <ContainerMsgsType>
        <Row title="delegator" value={<Account address={value.delegator_address} />} />
        <Row title="validator" value={<Account address={value.validator_address} />} />
        <Row
          title="self delegation"
          value={<AmountDenom amountValue={value.min_self_delegation} denom={BASE_DENOM} />}
        />
        <Row title="commission" value={value.commission.rate} />
        <Row title="description" value={value.description.moniker} />
      </ContainerMsgsType>
    );
  }
  if (type.includes('MsgEditValidator')) {
    return (
      <ContainerMsgsType>
        <Row title="address" value={<Account address={value.address} />} />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgUndelegate')) {
    return (
      <ContainerMsgsType>
        <Row title="delegator" value={<Account address={value.delegator_address} />} />
        <Row title="validator" value={<Account address={value.validator_address} />} />
        <Row
          title="amount"
          value={<AmountDenom amountValue={value.amount.amount} denom={value.amount.denom} />}
        />
      </ContainerMsgsType>
    );
  }
  if (type.includes('MsgBeginRedelegate')) {
    return (
      <ContainerMsgsType>
        <Row title="delegator" value={<Account address={value.delegator_address} />} />
        <Row title="from" value={<Account address={value.validator_src_address} />} />
        <Row title="to" value={<Account address={value.validator_dst_address} />} />
        <Row
          title="amount"
          value={<AmountDenom amountValue={value.amount.amount} denom={value.amount.denom} />}
        />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgWithdrawDelegatorReward')) {
    return (
      <ContainerMsgsType>
        <Row title="delegator" value={<Account address={value.delegator_address} />} />
        <Row title="validator" value={<Account address={value.validator_address} />} />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgWithdrawValidatorCommission')) {
    return (
      <ContainerMsgsType>
        <Row title="address" value={<Account address={value.validator_address} />} />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgModifyWithdrawAddress')) {
    return (
      <ContainerMsgsType>
        <Row title="delegator" value={<Account address={value.delegator_address} />} />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgUnjail')) {
    return (
      <ContainerMsgsType>
        <Row title="address" value={<Account address={value.address} />} />
      </ContainerMsgsType>
    );
  }

  // gov
  if (type.includes('MsgSubmitProposal')) {
    return (
      <ContainerMsgsType>
        <Row title="Proposer" value={<Account address={value.proposer} />} />
        <Row
          title="Initail Deposit"
          value={
            value.initial_deposit.length > 0 ? (
              value.initial_deposit.map((amount, i) => {
                if (i > 0) {
                  return (
                    <>
                      {' '}
                      ,
                      <AmountDenom amountValue={amount.amount} denom={amount.denom} />
                    </>
                  );
                }
                return <AmountDenom key={i} amountValue={amount.amount} denom={amount.denom} />;
              })
            ) : (
              <AmountDenom amountValue={0} denom={BASE_DENOM} />
            )
          }
        />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgDeposit')) {
    return (
      <ContainerMsgsType>
        <Row
          title="proposal"
          value={<Link to={`/senate/${value.proposal_id}`}>{value.proposal_id}</Link>}
        />
        <Row title="depositor" value={<Account address={value.depositor} />} />
        <Row
          title="amount"
          value={value.amount.map((amount, i) => {
            if (i > 0) {
              return (
                <>
                  {' '}
                  ,
                  <AmountDenom amountValue={amount.amount} denom={amount.denom} />
                </>
              );
            }
            return <AmountDenom amountValue={amount.amount} denom={amount.denom} key={i} />;
          })}
        />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgVote')) {
    return (
      <ContainerMsgsType>
        <Row
          title="proposal"
          value={<Link to={`/senate/${value.proposal_id}`}>{value.proposal_id}</Link>}
        />
        <Row title="voter" value={<Account address={value.voter} />} />
        <Row title="vote" value={value.option} />
      </ContainerMsgsType>
    );
  }

  // grid
  if (type.includes('MsgCreateRoute')) {
    return <MsgCreateRoute value={value} />;
  }

  if (type.includes('MsgEditRoute')) {
    return <MsgEditRoute value={value} />;
  }

  if (type.includes('MsgDeleteRoute')) {
    return <MsgDeleteRoute value={value} />;
  }

  if (type.includes('MsgEditRouteName')) {
    return <MsgCreateRoute value={value} />;
  }

  // ibc
  if (type.includes('MsgTransfer')) {
    return (
      <ContainerMsgsType>
        <Row title="from" value={<Account address={value.sender} />} />
        <Row title="to" value={<Account address={value.receiver} />} />
        <Row title="src channel" value={value.source_channel} />
        <Row
          title="token"
          value={<AmountDenom amountValue={value.token.amount} denom={value.token.denom} />}
        />
        <Row
          title="timeout"
          value={`${formatNumber(
            value.timeout_height.revision_height
          )} / ${formatNumber(value.timeout_height.revision_number)}`}
        />
      </ContainerMsgsType>
    );
  }

  // wasm
  if (type.includes('MsgInstantiateContract')) {
    return (
      <ContainerMsgsType>
        <Row title="address" value={<Account address={value.sender} />} />
        <Row title="label" value={value.label} />
        {value.code_id && <Row title="code id" value={value.code_id} />}
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgStoreCode')) {
    return (
      <ContainerMsgsType>
        <Row title="address" value={<Account address={value.sender} />} />
      </ContainerMsgsType>
    );
  }

  if (type.includes('MsgExecuteContract')) {
    return (
      <ContainerMsgsType>
        <Row title="address" value={<Account address={value.sender} />} />
        <Row title="contract" value={<Account address={value.contract} />} />
      </ContainerMsgsType>
    );
  }

  return <div>{type}</div>;
}

export default RenderValue;

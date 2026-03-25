import { Pane } from '@cybercongress/gravity';
import link from '../../image/link.svg';
import bank from '../../image/send.svg';
import stake from '../../image/stake.svg';
import gov from '../../image/gov.svg';
import distribution from '../../image/distribution.svg';
import slashing from '../../image/slashing.svg';
import investmint from '../../image/sync-outline.svg';
import grid from '../../image/share-social-outline.svg';
import swap from '../../image/swap-horizontal.svg';
import addLiquidity from '../../image/msgs_ic_pooladd.svg';
import subLiquidity from '../../image/msgs_ic_poolremove.svg';
import createPool from '../../image/flask-outline.svg';

function ContainerTitle({ img, children }) {
  return (
    <Pane display="flex" alignItems="center">
      {img && (
        <img
          alt={children}
          style={{
            width: '30px',
            height: '30px',
            marginRight: '5px',
          }}
          src={img}
        />
      )}
      {children}
    </Pane>
  );
}

function MsgType({ type }) {
  // if (type.includes('Link')) {
  //   return <ContainerTitle img={link}>Link</ContainerTitle>;
  // }

  if (type.includes('MsgCyberlink')) {
    return <ContainerTitle img={link}>Cyberlink</ContainerTitle>;
  }

  // investmint

  if (type.includes('MsgInvestmint')) {
    return <ContainerTitle img={investmint}>Investmint</ContainerTitle>;
  }

  // grid

  if (type.includes('MsgCreateRoute')) {
    return <ContainerTitle img={grid}>Create Route</ContainerTitle>;
  }
  if (type.includes('MsgEditRoute')) {
    return <ContainerTitle img={grid}>Edit Route</ContainerTitle>;
  }
  if (type.includes('MsgDeleteRoute')) {
    return <ContainerTitle img={grid}>Delete Route</ContainerTitle>;
  }
  if (type.includes('MsgEditRouteName')) {
    return <ContainerTitle img={grid}>Edit Route Name</ContainerTitle>;
  }

  // swap
  if (type.includes('MsgSwapWithinBatch')) {
    return <ContainerTitle img={swap}>Swap</ContainerTitle>;
  }
  if (type.includes('MsgDepositWithinBatch')) {
    return <ContainerTitle img={addLiquidity}>Add Liquidity</ContainerTitle>;
  }
  if (type.includes('MsgWithdrawWithinBatch')) {
    return <ContainerTitle img={subLiquidity}>Sub Liquidity</ContainerTitle>;
  }
  if (type.includes('MsgCreatePool')) {
    return <ContainerTitle img={createPool}>Create Pool</ContainerTitle>;
  }

  // bank

  if (type.includes('MsgSend')) {
    return <ContainerTitle img={bank}>Send</ContainerTitle>;
  }
  if (type.includes('MsgMultiSend')) {
    return <ContainerTitle img={bank}>Multi Send</ContainerTitle>;
  }
  if (type.includes('Receive')) {
    return <ContainerTitle img={bank}>Receive</ContainerTitle>;
  }

  // staking
  if (type.includes('MsgCreateValidator')) {
    return <ContainerTitle img={stake}>Create Validator</ContainerTitle>;
  }

  if (type.includes('MsgEditValidator')) {
    return <ContainerTitle img={stake}>Edit Validator</ContainerTitle>;
  }
  if (type.includes('MsgDelegate')) {
    return <ContainerTitle img={stake}>Delegate</ContainerTitle>;
  }
  if (type.includes('MsgUndelegate')) {
    return <ContainerTitle img={stake}>Undelegate</ContainerTitle>;
  }

  if (type.includes('MsgBeginRedelegate')) {
    return <ContainerTitle img={stake}>Redelegate</ContainerTitle>;
  }

  // gov
  if (type.includes('MsgSubmitProposal')) {
    return <ContainerTitle img={gov}>Submit Proposal</ContainerTitle>;
  }

  if (type.includes('MsgDeposit')) {
    return <ContainerTitle img={gov}>Deposit</ContainerTitle>;
  }
  if (type.includes('MsgVote')) {
    return <ContainerTitle img={gov}>Vote</ContainerTitle>;
  }

  // distribution
  if (type.includes('MsgWithdrawValidatorCommission')) {
    return <ContainerTitle img={distribution}>Withdraw Commission</ContainerTitle>;
  }

  if (type.includes('MsgWithdrawDelegatorReward')) {
    return <ContainerTitle img={distribution}>Withdraw Reward</ContainerTitle>;
  }
  if (type.includes('MsgModifyWithdrawAddress')) {
    return <ContainerTitle img={distribution}>Modify Withdraw Address</ContainerTitle>;
  }

  // slashing
  if (type.includes('MsgUnjail')) {
    return <ContainerTitle img={slashing}>Unjail</ContainerTitle>;
  }

  // ibc

  if (type.includes('MsgUpdateClient')) {
    return <ContainerTitle img={investmint}>IBC Update Client</ContainerTitle>;
  }

  if (type.includes('MsgRecvPacket')) {
    return <ContainerTitle img={investmint}>IBC Recv Packet</ContainerTitle>;
  }

  if (type.includes('MsgAcknowledgement')) {
    return <ContainerTitle img={investmint}>IBC Ack Packet</ContainerTitle>;
  }

  if (type.includes('MsgTransfer')) {
    return <ContainerTitle img={bank}>IBC Transfer</ContainerTitle>;
  }

  // wasm

  if (type.includes('MsgExecuteContract')) {
    return <ContainerTitle img={bank}>Execute Program</ContainerTitle>;
  }

  if (type.includes('MsgInstantiateContract')) {
    return <ContainerTitle img={bank}>Instantiate Program</ContainerTitle>;
  }

  if (type.includes('MsgStoreCode')) {
    return <ContainerTitle img={bank}>Store Program Code</ContainerTitle>;
  }

  if (type.includes('MsgMigrateContract')) {
    return <ContainerTitle img={bank}>Migrate Program</ContainerTitle>;
  }

  if (type.includes('MsgUpdateAdmin')) {
    return <ContainerTitle img={bank}>Update Program Admin</ContainerTitle>;
  }

  if (type.includes('MsgClearAdmin')) {
    return <ContainerTitle img={bank}>Clear Program Admin</ContainerTitle>;
  }

  return <div>{type}</div>;
}

export default MsgType;

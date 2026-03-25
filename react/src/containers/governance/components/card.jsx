import { Pane } from '@cybercongress/gravity';
import { ProposalStatus } from 'cosmjs-types/cosmos/gov/v1beta1/gov';
import Display from 'src/components/containerGradient/Display/Display';
import { IconStatus, Tooltip, Votes } from '../../../components';

const textPropsImg = require('../../../image/reader-outline.svg');
const paramChangePropsImg = require('../../../image/cog-outline.svg');
const comPropsImg = require('../../../image/wallet-outline.svg');

const finalTallyResult = (item) => {
  const finalVotes = {
    yes: 0,
    no: 0,
    abstain: 0,
    noWithVeto: 0,
  };
  let finalTotalVotes = 0;
  const yes = parseFloat(item.yes_count);
  const abstain = parseFloat(item.abstain_count);
  const no = parseFloat(item.no_count);
  const noWithVeto = parseFloat(item.no_with_veto_count);

  finalTotalVotes = yes + abstain + no + noWithVeto;
  if (finalTotalVotes !== 0) {
    finalVotes.yes = (yes / finalTotalVotes) * 100;
    finalVotes.no = (no / finalTotalVotes) * 100;
    finalVotes.abstain = (abstain / finalTotalVotes) * 100;
    finalVotes.noWithVeto = (noWithVeto / finalTotalVotes) * 100;
  }
  return finalVotes;
};

const returnResponseTypeObj = (typeImg, textType) => ({
  typeImg,
  textType,
});

const checkTypeProps = (type = '') => {
  if (type.includes('ParameterChangeProposal')) {
    return returnResponseTypeObj(paramChangePropsImg, 'Parameter Change Proposal');
  }
  if (type.includes('CommunityPoolSpendProposal')) {
    return returnResponseTypeObj(comPropsImg, 'Community Pool Spend Proposal');
  }

  if (type.includes('SoftwareUpgradeProposal')) {
    return returnResponseTypeObj(paramChangePropsImg, 'Software Upgrade Proposal');
  }

  if (type.includes('TextProposal')) {
    return returnResponseTypeObj(textPropsImg, 'Text Proposal');
  }

  if (type.includes('ClientUpdateProposal')) {
    return returnResponseTypeObj(paramChangePropsImg, 'Client Update Proposal');
  }

  return returnResponseTypeObj(textPropsImg, type);
};

function TypeProps({ type }) {
  const { typeImg, textType } = checkTypeProps(type);

  return (
    <Tooltip
      placement="bottom"
      tooltip={
        <div
          style={{
            padding: '10px',
            whiteSpace: 'nowrap',
          }}
        >
          {textType}
        </div>
      }
      style={{ paddingBottom: '20px' }}
    >
      <img style={{ width: 25, height: 25 }} src={typeImg} alt="type" />
    </Tooltip>
  );
}

function AcceptedCard({ id, name, votes, type, timeEnd }) {
  return (
    <Display>
      <Pane position="absolute" right="5px" top="5px">
        <TypeProps type={type} />
      </Pane>
      <Pane fontSize="20px" marginTop="10px" marginBottom={15}>
        #{id} {name}
      </Pane>
      <Pane marginBottom={5}>
        <Pane marginBottom={2}>Votes:</Pane>
        <Votes finalVotes={finalTallyResult(votes)} />
      </Pane>
      <Pane>
        <Pane marginBottom={2}>Time accepted:</Pane>
        <Pane>{timeEnd}</Pane>
      </Pane>
    </Display>
  );
}

function RejectedCard({ id, name, votes, type, timeEnd }) {
  return (
    <Display color="red">
      <Pane position="absolute" right="5px" top="5px">
        <TypeProps type={type} />
      </Pane>
      <Pane fontSize="20px" marginTop="10px" marginBottom={15}>
        #{id} {name}
      </Pane>
      <Pane marginBottom={5}>
        <Pane marginBottom={2}>Votes:</Pane>
        <Votes finalVotes={finalTallyResult(votes)} />
      </Pane>
      <Pane>
        <Pane marginBottom={2}>Time rejected:</Pane>
        <Pane>{timeEnd}</Pane>
      </Pane>
    </Display>
  );
}

function ActiveCard({ id, name, state, type = '', timeEndDeposit, timeEndVoting }) {
  return (
    <Display color="blue">
      <Pane position="absolute" right="5px" top="5px">
        <TypeProps type={type} />
      </Pane>
      <Pane fontSize="20px" marginTop="10px" marginBottom={15}>
        #{id} {name}
      </Pane>
      <Pane marginBottom={2}>
        <Pane alignItems="center" display="flex" marginBottom={2}>
          State:
          <IconStatus text marginLeft={10} marginRight={5} size={25} status={state} />
        </Pane>
      </Pane>

      {state === ProposalStatus.PROPOSAL_STATUS_DEPOSIT_PERIOD && (
        <Pane>
          <Pane marginBottom={2}>Deposit End Time:</Pane>
          <Pane>{timeEndDeposit}</Pane>
        </Pane>
      )}
      {state === ProposalStatus.PROPOSAL_STATUS_VOTING_PERIOD && (
        <Pane>
          <Pane marginBottom={2}>Voting End Time:</Pane>
          <Pane>{timeEndVoting}</Pane>
        </Pane>
      )}
    </Display>
  );
}

export { AcceptedCard, ActiveCard, RejectedCard };

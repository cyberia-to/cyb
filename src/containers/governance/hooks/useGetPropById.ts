import { Coin } from '@cosmjs/proto-signing';
import { ProposalStatus } from '@cybercongress/cyber-ts/cosmos/gov/v1/gov';
import { useQuery } from '@tanstack/react-query';
import { getProposalsDetail } from 'src/utils/governance';

type KeyOfProposalStatus = keyof typeof ProposalStatus;

const reduceChanges = (messages: any) => {
  if (messages.params) {
    return messages.params;
  }

  if (messages.content?.changes) {
    const { changes } = messages.content;
    return changes.reduce((acc, item) => {
      return { ...acc, [item.key]: item.value };
    }, {});
  }

  return undefined;
};

const decodeProps = (proposal: any) => {
  const { title, summary, status, messages } = proposal;

  const type = messages[0].content ? messages[0].content['@type'] : messages[0]['@type'];

  const proposer = !proposal.proposer.length ? messages[0].authority : proposal.proposer;

  const plan = messages[0].content ? messages[0].content.plan : undefined;
  const recipient = messages[0].content ? messages[0].content.recipient : undefined;

  const amount = messages[0].content?.amount ? messages[0].content.amount[0] : undefined;

  return {
    type,
    status: ProposalStatus[status as KeyOfProposalStatus],
    title: title || '<not set>',
    summary,
    totalDeposit: proposal.total_deposit[0],
    proposer,
    changes: reduceChanges(messages[0]),
    plan,
    recipient,
    amount,
  };
};

type ProposalsById = {
  status: ProposalStatus;
  type: string;
  title: string;
  summary: string;
  totalDeposit: Coin;
  proposer: string;
  recipient?: string;
  amount?: Coin;
  changes?: string;
  plan?: any;
};

function useGetPropById(proposalId?: string): {
  isLoading: boolean;
  data?: ProposalsById;
  refetch: () => void;
} {
  const { data, isLoading, refetch } = useQuery(
    ['getProposalsDetail', proposalId],
    async () => {
      return getProposalsDetail(proposalId);
    },
    { enabled: Boolean(proposalId) }
  );

  return { isLoading, data: data ? decodeProps(data) : undefined, refetch };
}

export default useGetPropById;

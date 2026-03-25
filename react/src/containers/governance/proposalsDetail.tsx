/* eslint-disable react/no-children-prop */

import { Pane, Text } from '@cybercongress/gravity';
import ReactMarkdown from 'react-markdown';
import { useLocation, useParams } from 'react-router-dom';
import rehypeSanitize from 'rehype-sanitize';
import remarkGfm from 'remark-gfm';
import { Account, Display, FormatNumberTokens, IconStatus, Item } from 'src/components';
import Loader2 from 'src/components/ui/Loader2';
import { useGovParam } from 'src/hooks/governance/params/useGovParams';
import { stringifyJson } from 'src/utils/json';
import ActionBarDetail from './actionBarDatail';
import useGetPropById from './hooks/useGetPropById';
import useTallyResult from './hooks/useTallyResult';
import styles from './proposalsDetail.module.scss';
import ProposalsDetailProgressBar from './proposalsDetailProgressBar';
import ProposalsRoutes from './proposalsRoutes';

const getSubStr = (str: string) => {
  let string = str;
  if (string.indexOf('cosmos-sdk/') !== -1) {
    string = string.slice(string.indexOf('/') + 1);
    return string;
  }
  return string;
};

function ProposalsDetail() {
  const { proposalId } = useParams();
  const location = useLocation();
  const { tallying, tallyResult, refetch: refetchTally } = useTallyResult(proposalId);

  const {
    data: proposals,
    isLoading: isLoadingProp,
    refetch: refetchPropById,
  } = useGetPropById(proposalId);

  const { paramData: minDeposit } = useGovParam('deposit');

  const refetch = () => {
    refetchTally();
    refetchPropById();
  };

  if (isLoadingProp) {
    return <Loader2 />;
  }

  return (
    <>
      <Pane display="flex" alignItems="center" marginBottom={20}>
        <Text fontSize="25px" color="#fff">
          {proposals && ` #${proposalId} ${proposals.title}`}
        </Text>
      </Pane>
      <Pane>
        <IconStatus status={proposals?.status || 0} text marginRight={8} />
      </Pane>

      <br />
      <Display>
        {/* fix, should be something */}

        <Item
          marginBottom={15}
          title="Proposer"
          value={<Account address={proposals?.proposer || ''} avatar />}
        />

        <Item marginBottom={15} title="Type" value={getSubStr(proposals?.type || '')} />

        {proposals?.recipient && (
          <Item
            title="Recipient"
            marginBottom={15}
            value={<Account address={proposals.recipient || ''} avatar />}
          />
        )}
        {proposals?.amount && (
          <Item
            title="Amount"
            marginBottom={15}
            value={
              <FormatNumberTokens
                text={proposals.amount.denom}
                value={parseFloat(proposals.amount.amount)}
              />
            }
          />
        )}

        <Item
          title="Description"
          value={
            <Pane className={styles.containerDescription}>
              <ReactMarkdown rehypePlugins={[rehypeSanitize]} remarkPlugins={[remarkGfm]}>
                {proposals ? proposals.summary.replace(/\\n/g, '\n') : ''}
              </ReactMarkdown>
            </Pane>
          }
        />

        {proposals?.changes && (
          <Item
            title="Changes"
            value={
              <Pane className={styles.containerDescription}>
                {Object.entries(proposals.changes).map(([key, value]) => (
                  <Pane key={key}>
                    {key}: {JSON.stringify(value, stringifyJson)}
                  </Pane>
                ))}
              </Pane>
            }
          />
        )}
        {proposals?.plan && (
          <Item
            title="Plan"
            value={
              <Pane className={styles.containerDescription}>
                <Pane>name: {proposals.plan.name}</Pane>
                <Pane>height: {proposals.plan.height}</Pane>
              </Pane>
            }
          />
        )}
      </Display>
      <ProposalsDetailProgressBar
        proposals={proposals}
        totalDeposit={proposals?.totalDeposit.amount || 0}
        minDeposit={minDeposit}
        tallying={tallying}
        tally={tallyResult}
      />

      <ProposalsRoutes
        proposals={proposals}
        tallying={tallying}
        tally={tallyResult}
        totalDeposit={proposals?.totalDeposit.amount || 0}
        updateFunc={refetch}
      />
      {location.pathname === `/senate/${proposalId}/voters` && (
        <ActionBarDetail id={proposalId} proposals={proposals} update={refetch} />
      )}
    </>
  );
}

export default ProposalsDetail;

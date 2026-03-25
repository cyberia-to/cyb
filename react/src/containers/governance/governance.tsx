/* eslint-disable camelcase */

import { Pane } from '@cybercongress/gravity';
import { useQuery } from '@tanstack/react-query';
import { ProposalStatus } from 'cosmjs-types/cosmos/gov/v1beta1/gov';
import dateFormat from 'dateformat';
import React, { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import Loader2 from 'src/components/ui/Loader2';
import { BASE_DENOM, DENOM_LIQUID } from 'src/constants/config';
import { useQueryClient } from 'src/contexts/queryClient';
import { useAdviser } from 'src/features/adviser/context';
import { routes } from 'src/routes';
import { ActionBar, CardStatisics } from '../../components';
import { getProposals } from '../../utils/governance';
import { coinDecimals, formatNumber } from '../../utils/utils';
import { AcceptedCard, ActiveCard, RejectedCard } from './components/card';
import Columns from './components/columns';
import styles from './components/styles.module.scss';

type KeyOfProposalStatus = keyof typeof ProposalStatus;

function Statistics({
  communityPoolCyber,
  staked,
}: {
  communityPoolCyber: number;
  staked: number;
}) {
  return (
    <Pane
      marginTop={10}
      marginBottom={50}
      display="grid"
      gridTemplateColumns="repeat(auto-fit, minmax(250px, 1fr))"
      gridGap="20px"
    >
      <CardStatisics
        title={`Community pool, ${BASE_DENOM.toUpperCase()}`}
        value={formatNumber(Math.floor(communityPoolCyber))}
      />
      <Link to="/sphere">
        <CardStatisics title="% of staked BOOT" value={formatNumber(staked * 100)} link />
      </Link>
      <Link to="/network/bostrom/parameters">
        <CardStatisics title="Network parameters" value={53} link />
      </Link>
    </Pane>
  );
}

function ProposalWrapper({
  proposalId,
  children,
}: {
  proposalId: string;
  children: React.ReactNode;
}) {
  return (
    <Link key={proposalId} style={{ color: 'unset' }} to={`/senate/${proposalId}`}>
      {children}
    </Link>
  );
}

//  ProposalsData['proposals'] extends (infer U)[] ? U : never
const mapProposalToCard = (proposal: any) => {
  const {
    id,
    total_deposit,
    status,
    deposit_end_time,
    voting_end_time,
    final_tally_result,
    title,
    messages,
  } = proposal;

  const type = messages[0].content ? messages[0].content['@type'] : messages[0]['@type'];

  return {
    proposalId: id,
    title: title || '<not set>',
    totalDeposit: total_deposit,
    state: ProposalStatus[status as KeyOfProposalStatus],
    timeEndDeposit: deposit_end_time
      ? dateFormat(new Date(deposit_end_time), 'dd/mm/yyyy, HH:MM:ss')
      : undefined,
    timeEndVoting: voting_end_time
      ? dateFormat(new Date(voting_end_time), 'dd/mm/yyyy, HH:MM:ss')
      : undefined,
    amounts: total_deposit[0] || undefined,
    votes: final_tally_result || undefined,
    type,
  };
};

function Governance() {
  const queryClient = useQueryClient();
  const [communityPoolCyber, setCommunityPoolCyber] = useState(0);
  const [staked, setStaked] = useState(0);
  const [_isLoadingStatistics, setIsLoadingStatistics] = useState(true);
  const { setAdviser } = useAdviser();

  useEffect(() => {
    setAdviser(
      <>
        the place where community will hear you
        <br /> propose your idea here
      </>
    );
  }, [setAdviser]);

  useEffect(() => {
    const getStatistics = async () => {
      if (queryClient) {
        setIsLoadingStatistics(true);
        let communityPool = 0;
        const totalCyb: Record<string, number> = {};
        let stakedBoot = 0;

        const dataCommunityPool = await queryClient.communityPool();
        const { pool } = dataCommunityPool;
        if (dataCommunityPool !== null) {
          communityPool = coinDecimals(Math.floor(parseFloat(pool[0].amount)));
        }
        setCommunityPoolCyber(communityPool);

        const datagetTotalSupply = await queryClient.totalSupply();
        if (Object.keys(datagetTotalSupply).length > 0) {
          datagetTotalSupply.forEach((item) => {
            totalCyb[item.denom] = parseFloat(item.amount);
          });
        }
        if (totalCyb[BASE_DENOM] && totalCyb[DENOM_LIQUID]) {
          stakedBoot = totalCyb[DENOM_LIQUID] / totalCyb[BASE_DENOM];
        }
        setStaked(stakedBoot);
        setIsLoadingStatistics(false);
      }
    };
    getStatistics();
  }, [queryClient]);

  const { data: tableData = [], isLoading: isLoadingProposals } = useQuery(
    ['proposals'],
    async () => {
      const response = await getProposals();
      return response || [];
    }
  );

  const active = (tableData || [])
    .reverse()
    .filter(
      (item) =>
        ProposalStatus[item.status as KeyOfProposalStatus] < ProposalStatus.PROPOSAL_STATUS_PASSED
    )
    .map(mapProposalToCard)
    .map((item) => (
      <ProposalWrapper proposalId={item.proposalId!} key={`active_${item.proposalId}`}>
        <ActiveCard
          key={item.proposalId}
          id={item.proposalId}
          name={item.title}
          type={item.type}
          state={item.state}
          timeEndDeposit={item.timeEndDeposit}
          timeEndVoting={item.timeEndVoting}
        />
      </ProposalWrapper>
    ));

  const accepted = (tableData || [])
    .filter(
      (item) =>
        ProposalStatus[item.status as KeyOfProposalStatus] === ProposalStatus.PROPOSAL_STATUS_PASSED
    )
    .map(mapProposalToCard)
    .map((item) => (
      <ProposalWrapper proposalId={item.proposalId} key={`accepted_${item.proposalId}`}>
        <AcceptedCard
          key={item.proposalId}
          id={item.proposalId}
          name={item.title}
          votes={item.votes}
          type={item.type}
          timeEnd={item.timeEndDeposit}
        />
      </ProposalWrapper>
    ));

  const rejected = (tableData || [])
    .reverse()
    .filter(
      (item) =>
        ProposalStatus[item.status as KeyOfProposalStatus] > ProposalStatus.PROPOSAL_STATUS_PASSED
    )
    .map(mapProposalToCard)
    .map((item) => (
      <ProposalWrapper proposalId={item.proposalId} key={`rejected_${item.proposalId}`}>
        <RejectedCard
          key={item.proposalId}
          id={item.proposalId}
          name={item.title}
          votes={item.votes}
          type={item.type}
          timeEnd={item.timeEndVoting}
        />
      </ProposalWrapper>
    ));

  return (
    <>
      <Statistics communityPoolCyber={communityPoolCyber} staked={staked} />

      {isLoadingProposals ? (
        <Loader2 />
      ) : (
        <div className={styles.column}>
          <Columns title="Active">{active}</Columns>
          <Columns title="Accepted">{accepted}</Columns>
          <Columns title="Rejected">{rejected}</Columns>
        </div>
      )}

      <ActionBar
        button={{
          text: 'Propose',
          link: routes.senate.routes.new.path,
        }}
      />
    </>
  );
}

export default Governance;

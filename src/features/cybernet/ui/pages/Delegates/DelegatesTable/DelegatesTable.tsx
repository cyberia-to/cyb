/* eslint-disable react/no-unstable-nested-components */

import { createColumnHelper } from '@tanstack/react-table';
import { useMemo } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { Account } from 'src/components';
import Table from 'src/components/Table/Table';
import { tableIDs } from 'src/components/Table/tableIDs';
import { Delegator } from 'src/features/cybernet/types';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import IconsNumber from '../../../../../../components/IconsNumber/IconsNumber';
import { SubnetPreviewGroup } from '../../../components/SubnetPreview/SubnetPreview';
import { useCybernet } from '../../../cybernet.context';
import useCurrentAccountStake from '../../../hooks/useCurrentAccountStake';
import { useDelegates } from '../../../hooks/useDelegate';
import { cybernetRoutes } from '../../../routes';
import useCybernetTexts from '../../../useCybernetTexts';

type Props = {};

const columnHelper = createColumnHelper<Delegator>();

const columnsIds = {
  stake: 'stake',
  myStake: 'myStake',
};

function DelegatesTable({}: Props) {
  const currentAddress = useCurrentAddress();
  const { getText } = useCybernetTexts();

  const { pathname } = useLocation();
  // maybe use props, think
  const isMyLearnerPage = pathname.includes('learners/my');

  const stakeQuery = useCurrentAccountStake({
    skip: !isMyLearnerPage,
  });

  const { data, loading: isLoading } = useDelegates();

  const { selectedContract, subnetsQuery } = useCybernet();
  const contractName = selectedContract?.metadata?.name;

  function getTotalStake(nominators: Delegator['nominators']) {
    return nominators.reduce((acc, [, stake]) => acc + stake, 0);
  }

  function getMyStake(nominators: Delegator['nominators']) {
    return nominators.find(([address]) => address === currentAddress)?.[1];
  }

  // maybe rename
  function getYieldValue(value: number) {
    return Number(((value / 1_000_000_000) * 100 * 365).toFixed(2));
  }

  const _subnets = subnetsQuery.data || [];

  const navigate = useNavigate();

  const columns = useMemo(() => {
    const cols = [
      columnHelper.accessor('delegate', {
        header: getText('delegate'),
        enableSorting: false,
        cell: (info) => {
          const address = info.getValue();

          return (
            <Account
              address={address}
              avatar
              markCurrentAddress
              link={cybernetRoutes.delegator.getLink('pussy', contractName, address)}
            />
          );
        },
      }),

      columnHelper.accessor('return_per_giga', {
        header: 'yield',
        sortingFn: (rowA, rowB) => {
          const a = getYieldValue(rowA.original.return_per_giga.amount);
          const b = getYieldValue(rowB.original.return_per_giga.amount);

          return a - b;
        },
        cell: (info) => {
          const value = info.getValue();

          const yieldVal = getYieldValue(value.amount);
          return `${yieldVal}%`;
        },
      }),

      columnHelper.accessor('registrations', {
        header: getText('subnetwork', true),
        sortingFn: (rowA, rowB) => {
          const a = rowA.original.registrations.length;
          const b = rowB.original.registrations.length;

          return a - b;
        },
        cell: (info) => {
          const value = info.getValue();

          return (
            <div
              style={{
                display: 'flex',
                justifyContent: 'center',
              }}
            >
              <SubnetPreviewGroup uids={value} />
            </div>
          );
        },
      }),

      columnHelper.accessor('nominators', {
        header: 'teach power',
        id: columnsIds.stake,
        sortingFn: (rowA, rowB) => {
          const totalA = getTotalStake(rowA.original.nominators);
          const totalB = getTotalStake(rowB.original.nominators);

          return totalA - totalB;
        },
        cell: (info) => {
          const nominators = info.getValue();
          const total = getTotalStake(nominators);

          return <IconsNumber value={total} type="pussy" />;
        },
      }),
      columnHelper.accessor('nominators', {
        header: 'my stake',
        id: columnsIds.myStake,
        sortingFn: (rowA, rowB) => {
          const myStakeA = getMyStake(rowA.original.nominators) || 0;
          const myStakeB = getMyStake(rowB.original.nominators) || 0;

          return myStakeA - myStakeB;
        },
        cell: (info) => {
          const nominators = info.getValue();
          const myStake = getMyStake(nominators);

          if (!myStake) {
            return null;
          }
          return <IconsNumber value={myStake} type="pussy" />;
        },
      }),
    ];

    if (isMyLearnerPage) {
      // cols.push()
    }

    return cols;
  }, [getText, contractName, isMyLearnerPage, getMyStake, getTotalStake, getYieldValue]);

  // use 1 loop
  const myMentors =
    stakeQuery.data?.filter(({ stake }) => stake > 0)?.map((stake) => stake.hotkey) || [];

  const renderData = !isMyLearnerPage
    ? data
    : data?.filter((mentor) => myMentors.includes(mentor.delegate));

  return (
    <Table
      id={tableIDs.cyberver.delegates}
      onSelect={(row) => {
        const index = Number(row);
        const { owner } = renderData!.find((_, i) => i === index)!;

        navigate(cybernetRoutes.delegator.getLink('pussy', contractName, owner));
      }}
      columns={columns}
      data={renderData || []}
      isLoading={isLoading}
      initialState={{
        sorting: [
          {
            id: columnsIds[!isMyLearnerPage ? 'stake' : 'myStake'],
            desc: true,
          },
        ],
      }}
    />
  );
}

export default DelegatesTable;

/* eslint-disable react/no-unstable-nested-components */

import { createColumnHelper } from '@tanstack/react-table';
import { useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { Account, Cid, Tooltip } from 'src/components';
import CIDResolver from 'src/components/CIDResolver/CIDResolver';
import Table from 'src/components/Table/Table';
import { tableIDs } from 'src/components/Table/tableIDs';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import { routes } from 'src/routes';
import { trimString } from 'src/utils/utils';
import { SubnetInfo } from '../../../../types';
import SubnetPreview from '../../../components/SubnetPreview/SubnetPreview';
import { useCurrentContract, useCybernet } from '../../../cybernet.context';
import useDelegate from '../../../hooks/useDelegate';
import { routes as subnetRoutes } from '../../../routes';
import useCybernetTexts from '../../../useCybernetTexts';
import GradeSetterInput from '../../Subnet/GradeSetterInput/GradeSetterInput';
import { useSubnet } from '../../Subnet/subnet.context';

type Props = {
  data: SubnetInfo[];
};

const columnHelper = createColumnHelper<SubnetInfo>();

// don't know good name
function CurrentToMax({ value, maxValue }: { value: number; maxValue: number }) {
  return (
    <div>
      {value}{' '}
      <span
        style={{
          color: 'gray',
        }}
      >
        / {maxValue}
      </span>
    </div>
  );
}

function SubnetsTable({ data }: Props) {
  const navigate = useNavigate();

  const address = useCurrentAddress();

  const { grades, subnetQuery } = useSubnet();

  // debug
  const { averageGrades } = grades?.all || {};

  const { getText } = useCybernetTexts();

  const rootSubnet = subnetQuery?.data?.netuid === 0;

  const { data: d2 } = useDelegate(address);
  const myCurrentSubnetsJoined = d2?.registrations;

  const myAddressJoinedRootSubnet = myCurrentSubnetsJoined?.includes(0);

  const { selectedContract } = useCybernet();

  const { contractName, network } = useCurrentContract();

  const columns = useMemo(() => {
    const col = [
      columnHelper.accessor('metadata', {
        header: 'name',
        id: 'subnetName',
        cell: (info) => {
          const _value = info.getValue();

          const { netuid } = info.row.original;

          const isMySubnet = myCurrentSubnetsJoined?.includes(netuid);

          return (
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '0 7px',
              }}
            >
              <SubnetPreview subnetUID={netuid} withName />
              {isMySubnet && (
                <Tooltip tooltip={`you joined this ${getText('subnetwork')}`}>✅</Tooltip>
              )}
            </div>
          );
        },
      }),

      columnHelper.accessor('owner', {
        header: getText('subnetOwner'),
        enableSorting: false,
        cell: (info) => {
          const value = info.getValue();

          return (
            <Account
              address={value}
              avatar
              link={routes.neuron.getLink(value)}
              markCurrentAddress
              // link={'../delegators/' + info.getValue()}
            />
          );
        },
      }),

      columnHelper.accessor('metadata.particle', {
        header: 'teaser',
        id: 'teaser',
        size: 150,
        enableSorting: false,
        cell: (info) => {
          const cid = info.getValue();

          return <CIDResolver cid={cid} />;
        },
      }),

      columnHelper.accessor('metadata.description', {
        header: 'rules',
        id: 'rules',
        enableSorting: false,
        cell: (info) => {
          const cid = info.getValue();

          return <Cid cid={cid}>{trimString(cid, 3, 3)}</Cid>;
        },
      }),
    ];

    if (!rootSubnet) {
      col.push(
        // @ts-expect-error
        columnHelper.accessor('max_allowed_validators', {
          header: getText('validator', true),
          sortingFn: (rowA, rowB) => {
            const a = rowA.original.subnetwork_n;
            const b = rowB.original.subnetwork_n;

            return a - b;
          },
          cell: (info) => {
            const max = info.getValue();

            const current = info.row.original.subnetwork_n;

            return <CurrentToMax value={current >= max ? max : current} maxValue={max} />;
          },
        }),
        columnHelper.accessor('max_allowed_uids', {
          header: getText('miner', true),
          sortingFn: (rowA, rowB) => {
            const a = rowA.original.subnetwork_n;
            const b = rowB.original.subnetwork_n;

            return a - b;
          },
          cell: (info) => {
            const max = info.getValue();

            const current = info.row.original.subnetwork_n;
            const maxAllowedValidators = info.row.original.max_allowed_validators;

            const diff = current - maxAllowedValidators;

            return (
              <CurrentToMax value={diff >= 0 ? diff : 0} maxValue={max - maxAllowedValidators} />
            );
          },
        })
      );
    }

    if (rootSubnet) {
      col.push(
        // @ts-expect-error
        columnHelper.accessor('netuid', {
          header: 'Grade (average)',
          id: 'grade',
          sortingFn: (rowA, rowB) => {
            const a = averageGrades[rowA.original.netuid];
            const b = averageGrades[rowB.original.netuid];

            return a - b;
          },
          cell: (info) => {
            const uid = info.getValue();

            // // fix
            // if (!grades.all?.data) {
            //   return 0;
            // }

            const v = averageGrades[uid];

            return v;
          },
        })
      );

      if (myAddressJoinedRootSubnet) {
        col.push(
          // @ts-expect-error
          columnHelper.accessor('netuid', {
            header: 'Set grade',
            id: 'setGrade',
            enableSorting: false,
            cell: (info) => {
              const uid = info.getValue();
              return <GradeSetterInput uid={uid} />;
            },
          })
        );
      }
    }

    return col;
  }, [myCurrentSubnetsJoined, myAddressJoinedRootSubnet, averageGrades, rootSubnet, getText]);

  return (
    <Table
      id={tableIDs.cyberver.subnets}
      onSelect={(row) => {
        const { netuid } = data[row];

        navigate(subnetRoutes.subnet.getLink('pussy', contractName, netuid));
      }}
      columns={columns}
      data={data}
      // if 1 - root subnet
      enableSorting={data.length !== 1}
    />
  );
}

export default SubnetsTable;

/* eslint-disable react/no-unstable-nested-components */

import { createColumnHelper } from '@tanstack/react-table';
import { useMemo } from 'react';
import { Link } from 'react-router-dom';
import { Account, Tooltip } from 'src/components';
import IconsNumber from 'src/components/IconsNumber/IconsNumber';
import Table from 'src/components/Table/Table';
import { tableIDs } from 'src/components/Table/tableIDs';
import { useAppData } from 'src/contexts/appData';
import AdviserHoverWrapper from 'src/features/adviser/AdviserHoverWrapper/AdviserHoverWrapper';
import { Delegator, SubnetNeuron } from 'src/features/cybernet/types';
import { SubnetPreviewGroup } from 'src/features/cybernet/ui/components/SubnetPreview/SubnetPreview';
import { useCurrentContract, useCybernet } from 'src/features/cybernet/ui/cybernet.context';
import { useDelegates } from 'src/features/cybernet/ui/hooks/useDelegate';
import useCybernetTexts from 'src/features/cybernet/ui/useCybernetTexts';
import { checkIsMLVerse } from 'src/features/cybernet/ui/utils/verses';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import { routes } from 'src/routes';
import { cybernetRoutes } from '../../../../../routes';
import GradeSetterInput from '../../../GradeSetterInput/GradeSetterInput';
import { useCurrentSubnet } from '../../../subnet.context';
import colorStyles from '../../Weights/WeightsTable/temp.module.scss';
import { getColor } from '../../Weights/WeightsTable/WeightsTable';

type Props = {};

enum TableColumnIDs {
  uid = 'uid',
  hotkey = 'hotkey',
  stake = 'stake',
  trust = 'trust',
  lastRewards = 'lastRewards',
  jobDone = 'jobDone',
  grade = 'grade',
  setGrade = 'setGrade',
  registrations = 'registrations',
}

const columnHelper = createColumnHelper<SubnetNeuron>();

// need refacfor
const key = 'subnetNeuronViewedBlock';

function getKey(address: string) {
  return `${key}_${address}`;
}

type LSType = {
  // current address
  [key: string]: {
    // subnet
    [key: string]: {
      // neuron
      [key: string]: number;
    };
  };
};

function getData(address: string) {
  const data = localStorage.getItem(getKey(address));

  return data ? (JSON.parse(data) as LSType) : null;
}

function save(data: LSType, address: string) {
  localStorage.setItem(getKey(address), JSON.stringify(data));
}

function handleSave(neuron: string, subnetId: number, block: number, currentAddress: string) {
  let data = getData(currentAddress);

  if (!data) {
    data = {};
  }

  if (!data[currentAddress]) {
    data[currentAddress] = {};
  }

  data[currentAddress] = {
    ...data[currentAddress],
    [subnetId]: {
      ...data[currentAddress][subnetId],
      [neuron]: block,
    },
  };

  save(data, currentAddress);
}

function SubnetNeuronsTable({}: Props) {
  const {
    subnetQuery,
    addressRegisteredInSubnet,
    neuronsQuery,

    isRootSubnet,
    netuid,
    grades: {
      all: { averageGrades },
    },
  } = useCurrentSubnet();

  const { metadata } = subnetQuery?.data || {};

  const address = useCurrentAddress();

  const { block } = useAppData();

  const { selectedContract } = useCybernet();

  const { type } = useCurrentContract();
  const isMLVerse = checkIsMLVerse(type);

  const { getText } = useCybernetTexts();

  const { data: delegatesData } = useDelegates();

  const neurons = useMemo(() => {
    return neuronsQuery.data || [];
  }, [neuronsQuery.data]);

  const stakeByNeurons = useMemo(() => {
    const stakes = neurons.reduce<Record<string, number>>((acc, neuron) => {
      const { stake } = neuron;
      const total = stake.reduce((acc, s) => acc + s[1], 0);

      acc[neuron.uid] = total;

      return acc;
    }, {});

    return stakes;
  }, [neurons]);

  const registrationsByNeuron = useMemo(() => {
    return delegatesData?.reduce<Record<Delegator['delegate'], Delegator['registrations']>>(
      (acc, delegator) => {
        const { registrations, delegate } = delegator;

        acc[delegate] = registrations;

        return acc;
      },
      {}
    );
  }, [delegatesData]);

  const viewedBlocks = getData(address);
  const cur = viewedBlocks?.[address]?.[netuid];

  const columns = useMemo(() => {
    const cols = [
      columnHelper.accessor('uid', {
        header: getText('uid'),
        id: TableColumnIDs.uid,
        cell: (info) => {
          const uid = info.getValue();
          return uid;
        },
      }),
      columnHelper.accessor('hotkey', {
        header: getText(isRootSubnet ? 'rootValidator' : 'delegate'),
        id: TableColumnIDs.hotkey,
        size: 200,
        enableSorting: false,
        cell: (info) => {
          const hotkey = info.getValue();
          const { validator_permit: validatorPermit } = info.row.original;

          const isProfessor = !!validatorPermit;

          return (
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '0 7px',
              }}
            >
              <Account
                address={hotkey}
                avatar
                markCurrentAddress
                link={cybernetRoutes.delegator.getLink(
                  'pussy',
                  selectedContract?.metadata?.name,
                  hotkey
                )}
              />

              {isProfessor && (
                <AdviserHoverWrapper adviserContent="this neuron is professor">
                  <span>💼</span>
                </AdviserHoverWrapper>
              )}
            </div>
          );
        },
      }),

      columnHelper.accessor('uid', {
        header: 'teach power',
        id: TableColumnIDs.stake,
        sortingFn: (rowA, rowB) => {
          const a = stakeByNeurons[rowA.original.uid];
          const b = stakeByNeurons[rowB.original.uid];

          return a - b;
        },
        cell: (info) => {
          const uid = info.getValue();
          const total = stakeByNeurons[uid];

          return <IconsNumber value={total} type="pussy" />;
        },
      }),
    ];

    if (!isRootSubnet) {
      cols.push(
        // @ts-expect-error
        columnHelper.accessor('hotkey', {
          header: 'job done',
          id: TableColumnIDs.jobDone,
          enableSorting: false,
          cell: (info) => {
            const hotkey = info.getValue();

            if (!metadata) {
              return null;
            }

            const viewedBlock = cur?.[hotkey];

            return (
              <>
                <Link
                  onClick={() => {
                    if (!block) {
                      return;
                    }

                    handleSave(hotkey, netuid, +block, address);
                  }}
                  to={`${routes.oracle.ask.getLink(
                    metadata.particle
                  )}?neuron=${hotkey}&subnet=${netuid}`}
                >
                  <Tooltip tooltip={`check what job have been done by this ${getText('delegate')}`}>
                    🔍
                  </Tooltip>
                </Link>

                <br />
                {viewedBlock && block && (
                  <span
                    style={{
                      fontSize: 14,
                    }}
                  >
                    (viewed {block - viewedBlock} blocks ago)
                  </span>
                )}
              </>
            );
          },
        }),

        columnHelper.accessor('validator_trust', {
          header: 'trust',
          id: TableColumnIDs.trust,
          cell: (info) => {
            const validatorTrust = info.getValue();
            const formatted = ((validatorTrust / 65536) * 100).toFixed(2);

            return `${formatted}%`;
          },
        }),
        columnHelper.accessor('emission', {
          header: 'last rewards',
          id: TableColumnIDs.lastRewards,
          cell: (info) => {
            const emission = info.getValue();
            return <IconsNumber value={emission} type="pussy" />;
          },
        }),

        columnHelper.accessor('uid', {
          header: 'grade',
          id: TableColumnIDs.grade,
          sortingFn: (rowA, rowB) => {
            const a = averageGrades[rowA.original.uid];
            const b = averageGrades[rowB.original.uid];

            return a - b;
          },
          cell: (info) => {
            const uid = info.getValue();

            // if (!allGrades) {
            //   return null;
            // }

            const avg = averageGrades[uid];

            const color = getColor(avg);

            return <span className={colorStyles[`color_${color}`]}>{avg}</span>;
          },
        })
      );

      if (addressRegisteredInSubnet && !isMLVerse) {
        cols.push(
          // @ts-expect-error
          columnHelper.accessor('uid', {
            header: 'Set grade',
            id: TableColumnIDs.setGrade,
            enableSorting: false,
            cell: (info) => {
              const uid = info.getValue();

              return <GradeSetterInput key={uid} uid={uid} />;
            },
          })
        );
      }
    } else {
      // eslint-disable-next-line no-lonely-if
      if (registrationsByNeuron) {
        cols.push(
          // @ts-expect-error
          columnHelper.accessor('hotkey', {
            header: 'Registrations',
            id: TableColumnIDs.registrations,
            sortingFn: (rowA, rowB) => {
              const a = registrationsByNeuron[rowA.original.hotkey].length;
              const b = registrationsByNeuron[rowB.original.hotkey].length;

              return a - b;
            },
            cell: (info) => {
              const hotkey = info.getValue();
              const registrations = registrationsByNeuron[hotkey];

              const subnetsWithoutRoot = registrations.filter((r) => r !== 0);

              return (
                <div
                  style={{
                    display: 'flex',
                    justifyContent: 'center',
                  }}
                >
                  {subnetsWithoutRoot.length > 0 ? (
                    <SubnetPreviewGroup uids={subnetsWithoutRoot} />
                  ) : (
                    '-'
                  )}
                </div>
              );
            },
          })
        );
      }
    }

    return cols;
  }, [
    averageGrades,
    isMLVerse,
    stakeByNeurons,
    getText,
    registrationsByNeuron,
    selectedContract,
    metadata,
    netuid,
    addressRegisteredInSubnet,
    isRootSubnet,
    address,
    block,
    cur,
    // block,
    // cur,
  ]);

  let order;
  if (isRootSubnet) {
    order = [
      TableColumnIDs.uid,
      TableColumnIDs.hotkey,
      TableColumnIDs.registrations,
      TableColumnIDs.stake,
    ];
  } else {
    order = [
      TableColumnIDs.uid,
      TableColumnIDs.hotkey,
      TableColumnIDs.jobDone,
      TableColumnIDs.stake,
      TableColumnIDs.lastRewards,
      TableColumnIDs.trust,
      TableColumnIDs.grade,
      TableColumnIDs.setGrade,
    ];
  }

  return (
    <Table
      id={tableIDs.cyberver.subnetNeurons}
      columns={columns}
      data={neurons}
      isLoading={neuronsQuery?.loading}
      initialState={{
        columnOrder: order,
        sorting: [
          {
            id: !isRootSubnet ? TableColumnIDs.grade : TableColumnIDs.stake,
            desc: true,
          },
        ],
      }}
    />
  );
}

export default SubnetNeuronsTable;

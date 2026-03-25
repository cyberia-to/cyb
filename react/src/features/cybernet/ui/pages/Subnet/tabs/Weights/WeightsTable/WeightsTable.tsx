/* eslint-disable react/no-unstable-nested-components */
import { createColumnHelper } from '@tanstack/react-table';
import { useMemo } from 'react';
import { Account } from 'src/components';
import Table from 'src/components/Table/Table';
import { SubnetNeuron } from 'src/features/cybernet/types';
import SubnetPreview from 'src/features/cybernet/ui/components/SubnetPreview/SubnetPreview';
import { useCurrentContract, useCybernet } from 'src/features/cybernet/ui/cybernet.context';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import { cybernetRoutes } from '../../../../../routes';
import { useSubnet } from '../../../subnet.context';
import colorStyles from './temp.module.scss';
import styles from './WeightsTable.module.scss';

type Props = {};

export function getColor(value) {
  let color;

  if (value < 3) {
    color = 'red';
  } else if (value < 6) {
    color = 'orange';
  } else {
    color = 'green';
  }

  return color;
}

const columnHelper = createColumnHelper<SubnetNeuron>();

function WeightsTable({}: Props) {
  const address = useCurrentAddress();

  const { subnetQuery, grades, neuronsQuery } = useSubnet();

  const { subnetsQuery } = useCybernet();

  const uid = subnetQuery.data?.netuid;
  const isRootSubnet = uid === 0;

  const neurons = useMemo(() => {
    return neuronsQuery.data || [];
  }, [neuronsQuery.data]);

  const currentContract = useCurrentContract();

  if (!neurons.length) {
    return null;
  }

  const columns = isRootSubnet
    ? subnetsQuery.data?.map((subnet) => subnet.netuid).filter((subnet) => subnet !== 0)
    : neurons.map((neuron) => neuron.uid);

  const { data } = grades.all;

  if (!columns?.length) {
    return null;
  }

  return (
    <div>
      <div className={styles.wrapper}>
        <Table
          enableSorting={false}
          data={neurons}
          columns={[
            // @ts-expect-error
            columnHelper.accessor('hotkey', {
              id: 'uid',
              header: '',
              cell: (info) => {
                const uid = info.getValue();

                return (
                  <Account
                    address={uid}
                    avatar
                    markCurrentAddress
                    link={cybernetRoutes.delegator.getLink(
                      currentContract.network,
                      currentContract.contractName,
                      uid
                    )}
                  />
                );
              },
            }),
          ]}
        />

        <Table
          enableSorting={false}
          columns={
            // useMemo(
            // () =>
            columns.map((uid) => {
              return columnHelper.accessor(String(uid), {
                id: `t${uid}`,
                header: () => {
                  if (isRootSubnet) {
                    return <SubnetPreview subnetUID={uid} withName />;
                  }

                  const { hotkey } = neurons[uid];

                  return (
                    <div className={styles.headerCell}>
                      {address === hotkey && (
                        <span
                          style={{
                            position: 'absolute',
                            top: 15,
                            left: 20,
                          }}
                        >
                          🔑
                        </span>
                      )}
                      <Account
                        address={hotkey}
                        avatar
                        // markCurrentAddress
                        onlyAvatar
                        link={cybernetRoutes.delegator.getLink(
                          currentContract.network,
                          currentContract.contractName,
                          hotkey
                        )}
                      />
                    </div>
                  );
                },
                cell: (info) => {
                  const val = info.row.original[uid];

                  if (!val) {
                    return '-';
                  }

                  const color = getColor(val);

                  return <div className={colorStyles[`color_${color}`]}>{val}</div>;
                },
              });
            })
            // [columns, address, isRootSubnet, neurons, currentContract]
            // )
          }
          data={data}
        />
      </div>
    </div>
  );
}

export default WeightsTable;

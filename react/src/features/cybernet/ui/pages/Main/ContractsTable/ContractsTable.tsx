/* eslint-disable react/no-unstable-nested-components */
import { createColumnHelper } from '@tanstack/react-table';
import { useMemo } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { Cid, DenomArr } from 'src/components';
import CIDResolver from 'src/components/CIDResolver/CIDResolver';
import Table from 'src/components/Table/Table';
import { AvataImgIpfs } from 'src/containers/portal/components/avataIpfs';
import { ContractTypes, ContractWithData } from 'src/features/cybernet/types';
import { routes } from 'src/routes';
import { trimString } from 'src/utils/utils';
import { useCybernet } from '../../../cybernet.context';
import { cybernetRoutes } from '../../../routes';
import styles from './ContractsTable.module.scss';

const columnHelper = createColumnHelper<ContractWithData>();

function ContractsTable() {
  const { contracts, selectedContract } = useCybernet();

  const navigate = useNavigate();

  return (
    <div className={styles.wrapper}>
      <Table
        onSelect={(row) => {
          if (!row) {
            return;
          }

          const contract = contracts[row!];
          const { address, metadata: { name } = {} } = contract;

          navigate(cybernetRoutes.verse.getLink('pussy', name || address));
        }}
        enableSorting={false}
        columns={useMemo(
          () => [
            columnHelper.accessor('metadata.name', {
              header: '',
              maxSize: 250,
              size: 0,
              cell: (info) => {
                const value = info.getValue();

                const { original } = info.row;
                const logo = original.metadata?.logo;

                const { address } = original;

                const selected = selectedContract?.address === address;

                return (
                  <div className={styles.nameCell}>
                    {selected && <span>✔</span>}

                    <Link to={cybernetRoutes.subnets.getLink('pussy', value || address)}>
                      {logo && <AvataImgIpfs cidAvatar={logo} />}
                      {value || trimString(address, 6, 3)}
                    </Link>
                  </div>
                );
              },
            }),

            columnHelper.accessor('metadata.particle', {
              header: '',
              size: 300,
              cell: (info) => {
                const value = info.getValue();
                const row = info.row.original;

                if (!row.metadata) {
                  return '-';
                }

                const { type } = row;
                const difficulty = type === ContractTypes.Graph ? 'easy' : 'hard';

                return (
                  <div className={styles.descriptionCell}>
                    <span className={styles.smallText}>{difficulty}:</span>{' '}
                    <p>
                      <CIDResolver
                        cid={value}
                        // fallback={<Cid cid={value}>info</Cid>}
                      />
                    </p>
                  </div>
                );
              },
            }),

            columnHelper.accessor('economy.staker_apr', {
              header: '',
              cell: (info) => {
                const value = info.getValue();

                if (!value) {
                  return '-';
                }

                return (
                  <span>
                    {Number(info.getValue()).toFixed(2)}
                    <span className={styles.smallText}>% teach yield</span>
                  </span>
                );
              },
            }),
            columnHelper.accessor('metadata.description', {
              header: '',
              cell: (info) => {
                const cid = info.getValue();

                if (!cid) {
                  return '-';
                }

                return <Cid cid={cid}>rules</Cid>;
              },
            }),
            columnHelper.accessor('address', {
              header: '',
              id: 'network',
              cell: (info) => {
                const _value = info.getValue();

                return <DenomArr type="network" denomValue="space-pussy" />;
              },
            }),
            columnHelper.accessor('address', {
              header: '',
              cell: (info) => {
                const value = info.getValue();

                if (value === '-') {
                  return '-';
                }

                return (
                  <Link to={routes.contracts.byAddress.getLink(value)}>
                    {trimString(value, 9, 3)}
                  </Link>
                );
              },
            }),
          ],
          [selectedContract]
        )}
        data={contracts}
      />
    </div>
  );
}

export default ContractsTable;

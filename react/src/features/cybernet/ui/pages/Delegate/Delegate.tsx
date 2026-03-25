/* eslint-disable react/no-unstable-nested-components */

import { createColumnHelper } from '@tanstack/react-table';
import { useParams } from 'react-router-dom';
import { Account, AmountDenom } from 'src/components';
import Display from 'src/components/containerGradient/Display/Display';
import DisplayTitle from 'src/components/containerGradient/DisplayTitle/DisplayTitle';
import IconsNumber from 'src/components/IconsNumber/IconsNumber';
import MusicalAddress from 'src/components/MusicalAddress/MusicalAddress';
import Table from 'src/components/Table/Table';
import AdviserHoverWrapper from 'src/features/adviser/AdviserHoverWrapper/AdviserHoverWrapper';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import { Delegator, Delegator as DelegatorType } from 'src/features/cybernet/types';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppSelector } from 'src/redux/hooks';
import { SubnetPreviewGroup } from '../../components/SubnetPreview/SubnetPreview';
import useDelegate from '../../hooks/useDelegate';
import useCybernetTexts from '../../useCybernetTexts';
import subnetStyles from '../Subnet/Subnet.module.scss';
import styles from './Delegate.module.scss';
import DelegateActionBar from './DelegateActionBar/DelegateActionBar';

const columnHelper = createColumnHelper<Delegator>();

const config: keyof DelegatorType = {
  take: {
    text: 'Commission',
  },
  validator_permits: {
    text: 'Validator permits',
  },
  total_daily_return: {
    text: 'Total daily return',
  },
  return_per_1000: {
    text: 'Return per 1000 🟣',
  },
};

function Delegator() {
  const { id } = useParams();

  const currentAddress = useAppSelector(selectCurrentAddress);

  const address = id !== 'my' ? id : currentAddress;

  const { loading, data, error, refetch } = useDelegate(address);
  const { getText } = useCybernetTexts();

  useAdviserTexts({
    isLoading: loading,
    loadingText: `loading ${getText('delegate')}`,
    error,
    defaultText: `${getText('delegate')} info`,
  });

  const myStake = data?.nominators.find(([address]) => address === currentAddress)?.[1];

  const nominators = data?.nominators;

  const totalStake = nominators?.reduce((acc, [, stake]) => acc + stake, 0);

  return (
    <>
      {myStake && data.delegate !== currentAddress && (
        <Display title={<DisplayTitle title="My stake" />}>
          <AmountDenom amountValue={myStake} denom="pussy" />
        </Display>
      )}

      <Display noPadding title={<DisplayTitle title={<MusicalAddress address={address} />} />}>
        {!loading && !data && (
          <div
            style={{
              textAlign: 'center',
            }}
          >
            no {getText('delegate')} found, or staking not enabled
          </div>
        )}

        <ul className={subnetStyles.list}>
          {data &&
            Object.keys(data)
              .filter((item) => !['nominators', 'delegate'].includes(item))
              .map((item) => {
                const value = data[item];
                let content = value;

                if (item === 'owner') {
                  content = <Account avatar address={value} markCurrentAddress />;
                }

                if (item === 'take') {
                  content = <span>{(value / 65535).toFixed(2) * 100}%</span>;
                }

                if (['total_daily_return', 'return_per_1000', 'return_per_giga'].includes(item)) {
                  content = (
                    <div>
                      <IconsNumber value={value.amount} type="pussy" />
                    </div>
                  );
                }

                if (item === 'registrations' || item === 'validator_permits') {
                  content = (
                    <ul className={styles.list}>
                      <SubnetPreviewGroup uids={value} />
                    </ul>
                  );
                }

                return (
                  <li key={item}>
                    {config[item]?.text || item}: {content}
                  </li>
                );
              })}
        </ul>
      </Display>

      {!!nominators?.length && (
        <Display
          noPadding
          title={
            <DisplayTitle
              title={
                <div className={styles.nominatorsHeader}>
                  <AdviserHoverWrapper adviserContent="learners">
                    <h3>{getText('delegator', true)}</h3>
                  </AdviserHoverWrapper>

                  <div>
                    <IconsNumber value={totalStake} type="pussy" />
                  </div>
                </div>
              }
            />
          }
        >
          <Table
            columns={[
              columnHelper.accessor('address', {
                header: getText('delegator'),
                enableSorting: false,
                cell: (info) => <Account address={info.getValue()} avatar markCurrentAddress />,
              }),
              columnHelper.accessor('amount', {
                header: 'teach power',
                cell: (info) => {
                  const value = info.getValue();

                  return (
                    <div
                      style={{
                        display: 'flex',
                        justifyContent: 'flex-end',
                      }}
                    >
                      <IconsNumber value={value} type="pussy" />
                    </div>
                  );
                },
              }),
            ]}
            data={nominators.map(([address, amount]) => {
              return {
                address,
                amount,
              };
            })}
            initialState={{
              sorting: [
                {
                  id: 'amount',
                  desc: true,
                },
              ],
            }}
          />
        </Display>
      )}

      <DelegateActionBar address={address} stakedAmount={myStake} onSuccess={refetch} />
    </>
  );
}

export default Delegator;

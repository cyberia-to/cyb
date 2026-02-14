import { useAppSelector } from 'src/redux/hooks';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useParams } from 'react-router-dom';

import { useSphereContext } from 'src/pages/Sphere/Sphere.context';
import { Tabs } from 'src/components';
import { useCallback } from 'react';
import { routes } from 'src/routes';
import Statistics from './components/Statistics/Statistics';
import Details from './components/Details/Details';
import reduceDetails from './utils/reduceDetails';
import useSelfDelegation from '../../../../../features/staking/delegation/useSelfDelegation';
import useDelegationRewards from '../../../../../features/staking/rewards/useDelegationRewards';
import useValidatorByContext from './hooks/useValidatorByContext';
import useValidatorDelegations from '../../../../../features/staking/delegation/useValidatorDelegations';
import ActionBarContainer from '../../components/ActionBarContainer/ActionBarContainer';
import Leadership from './components/Leadership/Leadership';
import Rumors from './components/Rumors/Rumors';
import SearchResultsTab from './components/SearchResultsTab/SearchResultsTab';

enum TabsKey {
  main = 'main',
  rumors = 'rumors',
  leadership = 'leadership',
  search = 'search',
}

function HeroDetails() {
  const addressActive = useAppSelector(selectCurrentAddress);
  const { address: operatorAddress, tab = TabsKey.main } = useParams();
  const { delegationsData, bondedTokens, stakingProvisions, refetchFunc } =
    useSphereContext();

  const { validatorInfo } = useValidatorByContext(operatorAddress);

  const { reward, refetch: refetchReward } = useDelegationRewards(
    addressActive,
    operatorAddress
  );

  const { delegations } = useValidatorDelegations(operatorAddress);

  const { selfDelegationCoin } = useSelfDelegation(operatorAddress);

  const staked =
    operatorAddress && delegationsData[operatorAddress]
      ? delegationsData[operatorAddress]
      : undefined;

  const updateFnc = useCallback(() => {
    refetchFunc();
    refetchReward();
  }, [refetchFunc, refetchReward]);

  if (!validatorInfo) {
    return <div>...</div>;
  }

  return (
    <>
      <Statistics
        data={{
          staked,
          reward,
          info: {
            ...validatorInfo.description,
            status: validatorInfo.status,
          },
        }}
      />

      <Tabs
        selected={tab}
        options={Object.keys(TabsKey).map((item) => ({
          key: item,
          to: routes.hero.getLinkToTab(
            validatorInfo.operatorAddress,
            item === TabsKey.main ? '' : item
          ),
        }))}
      />

      {tab === TabsKey.main && (
        <Details
          data={validatorInfo}
          options={{
            ...reduceDetails(
              validatorInfo,
              bondedTokens,
              selfDelegationCoin,
              stakingProvisions
            ),
            delegations,
          }}
        />
      )}
      {tab === TabsKey.rumors && (
        <Rumors accountUser={validatorInfo.operatorAddress} />
      )}
      {tab === TabsKey.leadership && (
        <Leadership accountUser={validatorInfo.operatorAddress} />
      )}

      {tab === TabsKey.search && (
        <SearchResultsTab moniker={validatorInfo.description.moniker} />
      )}

      {tab !== TabsKey.search && (
        <ActionBarContainer validators={validatorInfo} updateFnc={updateFnc} />
      )}
    </>
  );
}

export default HeroDetails;

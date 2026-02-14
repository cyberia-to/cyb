import { useEffect, useState } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';
import {
  BECH32_PREFIX_VALOPER,
  BASE_DENOM,
  DENOM_LIQUID,
} from 'src/constants/config';

import { getDelegatorDelegations } from 'src/features/staking/delegation/getDelegatorDelegations';
import { useCyberClient } from 'src/contexts/queryCyberClient';
import { coinDecimals, fromBech32 } from '../../../../../utils/utils';
import useGetSlots from '../../../../../containers/mint/useGetSlots';

const initValue = {
  available: 0,
  delegation: 0,
  unbonding: 0,
  rewards: 0,
  total: 0,
};

const initValueTokens = {
  liquid: 0,
  vested: 0,
  total: 0,
};

const initValueToken = {
  [DENOM_LIQUID]: { ...initValueTokens },
  milliampere: { ...initValueTokens },
  millivolt: { ...initValueTokens },
};

function useGetBalance(address, updateAddress) {
  const queryClient = useQueryClient();
  const { rpc } = useCyberClient();
  const [addressActive, setAddressActive] = useState(null);
  const [loadingBalanceInfo, setLoadingBalanceInfo] = useState(true);
  const [loadingBalanceToken, setLoadingBalanceToken] = useState(true);
  const [balance, setBalance] = useState(initValue);
  const [balanceToken, setBalanceToken] = useState(initValueToken);
  const { vested, originalVesting, loadingAuthAccounts } = useGetSlots(
    addressActive,
    updateAddress
  );

  useEffect(() => {
    if (address) {
      if (address.bech32) {
        setAddressActive(address.bech32);
      } else {
        setAddressActive(address);
      }
    }
  }, [address]);

  useEffect(() => {
    const getBalance = async () => {
      try {
        if (queryClient && addressActive !== null && rpc) {
          setBalance(initValue);
          setLoadingBalanceInfo(true);
          const availablePromise = await queryClient.getBalance(
            addressActive,
            BASE_DENOM
          );
          setBalance((item) => ({
            ...item,
            available: parseFloat(availablePromise.amount),
            total: item.total + parseFloat(availablePromise.amount),
          }));

          const delegationResponses = await getDelegatorDelegations(
            rpc,
            addressActive
          );
          let delegationsAmount = 0;

          if (delegationResponses.length) {
            delegationResponses.forEach((itemDelegation) => {
              delegationsAmount += parseFloat(itemDelegation.balance.amount);
            });
          }
          setBalance((item) => ({
            ...item,
            delegation: parseFloat(delegationsAmount),
            total: item.total + parseFloat(delegationsAmount),
          }));

          const unbondingPromise =
            await queryClient.delegatorUnbondingDelegations(addressActive);
          if (
            unbondingPromise.unbondingResponses &&
            unbondingPromise.unbondingResponses.length > 0
          ) {
            const { unbondingResponses } = unbondingPromise;
            unbondingResponses.forEach((unbond, i) => {
              unbond.entries.forEach((entry, j) => {
                setBalance((item) => ({
                  ...item,
                  unbonding: Math.floor(
                    item.unbonding + parseFloat(entry.balance)
                  ),
                  total: Math.floor(item.total + parseFloat(entry.balance)),
                }));
              });
            });
          }
          const rewardsPropsise = await queryClient.delegationTotalRewards(
            addressActive
          );
          if (rewardsPropsise.total && rewardsPropsise.total.length > 0) {
            setBalance((item) => ({
              ...item,
              rewards: Math.floor(
                coinDecimals(parseFloat(rewardsPropsise.total[0].amount))
              ),
              total: Math.floor(
                item.total +
                  coinDecimals(parseFloat(rewardsPropsise.total[0].amount))
              ),
            }));
          }
          const dataValidatorAddress = fromBech32(
            addressActive,
            BECH32_PREFIX_VALOPER
          );
          const resultGetDistribution = await queryClient.validatorCommission(
            dataValidatorAddress
          );
          if (resultGetDistribution.commission.commission.length > 0) {
            const { commission } = resultGetDistribution;
            setBalance((item) => ({
              ...item,
              commission: Math.floor(
                coinDecimals(parseFloat(commission.commission[0].amount))
              ),
              total: Math.floor(
                item.total +
                  coinDecimals(parseFloat(commission.commission[0].amount))
              ),
            }));
          }
        } else {
          setBalance(initValue);
        }
        setLoadingBalanceInfo(false);
      } catch (e) {
        console.log(e);
        setLoadingBalanceInfo(false);
        setBalance(initValue);
      }
    };
    getBalance();
  }, [queryClient, rpc, addressActive, updateAddress]);

  useEffect(() => {
    const getBalance = async () => {
      const initValueTokenAmount = {
        [DENOM_LIQUID]: {
          ...initValueTokens,
        },
        milliampere: {
          ...initValueTokens,
        },
        millivolt: {
          ...initValueTokens,
        },
        tocyb: 0,
      };

      if (queryClient && addressActive !== null && !loadingAuthAccounts) {
        setBalanceToken(initValueToken);
        setLoadingBalanceToken(true);
        const getAllBalancesPromise = await queryClient.getAllBalances(
          addressActive
        );
        const balancesToken = getCalculationBalance(getAllBalancesPromise);
        if (Object.keys(balancesToken).length > 0) {
          Object.keys(balancesToken).forEach((key) => {
            if (
              Object.hasOwnProperty.call(balancesToken, key) &&
              key !== BASE_DENOM
            ) {
              const elementBalancesToken = balancesToken[key];

              if (
                Object.hasOwnProperty.call(initValueTokenAmount, key) &&
                Object.hasOwnProperty.call(initValueTokenAmount[key], 'total')
              ) {
                initValueTokenAmount[key].total = elementBalancesToken;
                initValueTokenAmount[key].liquid = elementBalancesToken;
              } else {
                initValueTokenAmount[key] = elementBalancesToken;
              }
              if (
                Object.hasOwnProperty.call(originalVesting, key) &&
                Object.hasOwnProperty.call(vested, key)
              ) {
                const vestedTokens =
                  parseFloat(originalVesting[key]) - parseFloat(vested[key]);
                const liquidAmount = elementBalancesToken - vestedTokens;
                initValueTokenAmount[key].liquid =
                  liquidAmount > 0 ? liquidAmount : 0;
                initValueTokenAmount[key].vested = vestedTokens;
              }
            }
          });
        }
      }
      // console.log(`initValueTokenAmount`, initValueTokenAmount);
      setBalanceToken(initValueTokenAmount);
      setLoadingBalanceToken(false);
    };
    getBalance();
  }, [
    queryClient,
    addressActive,
    vested,
    originalVesting,
    loadingAuthAccounts,
  ]);

  const getCalculationBalance = (data) => {
    const balances = {};
    if (Object.keys(data).length > 0) {
      data.forEach((item) => {
        balances[item.denom] = parseFloat(item.amount);
      });
    }

    return balances;
  };

  return { balance, loadingBalanceInfo, balanceToken, loadingBalanceToken };
}

export default useGetBalance;

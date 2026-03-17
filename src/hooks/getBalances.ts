import { useCallback, useEffect, useRef, useState } from 'react';
import { DENOM_LIQUID } from 'src/constants/config';
import { useQueryClient } from 'src/contexts/queryClient';
import { authAccounts } from '../utils/search/utils';
import { reduceBalances } from '../utils/utils';

const MILLISECONDS_IN_SECOND = 1000;

const getVestingPeriodsData = (data, startTime) => {
  let length = parseFloat(startTime);
  const vestedAmount = {
    [DENOM_LIQUID]: 0,
    millivolt: 0,
    milliampere: 0,
  };

  if (data.length > 0) {
    data.forEach((item) => {
      let amount = {};
      length += parseFloat(item.length);
      const lengthMs = length * MILLISECONDS_IN_SECOND;
      const d = Date.parse(new Date());
      amount = reduceBalances(item.amount);
      if (lengthMs < d) {
        if (Object.keys(amount).length > 0) {
          // eslint-disable-next-line no-restricted-syntax
          for (const key in amount) {
            if (Object.hasOwn(amount, key)) {
              const element = amount[key];
              if (Object.hasOwn(vestedAmount, key)) {
                vestedAmount[key] += element;
              }
            }
          }
        }
      }
    });
  }

  return vestedAmount;
};

function useGetBalances(addressActive: string | undefined) {
  const queryClient = useQueryClient();
  const [allBalances, setAllBalances] = useState(null);
  const [vestedAmount, setVestedAmount] = useState(null);
  const [liquidBalances, setLiquidBalances] = useState(null);
  const [_update, setUpdate] = useState(0);

  const pollingRef = useRef<ReturnType<typeof setInterval>>();
  const snapshotRef = useRef<string | null>(null);

  const stopPolling = useCallback(() => {
    if (pollingRef.current) {
      clearInterval(pollingRef.current);
      pollingRef.current = undefined;
    }
  }, []);

  const fetchBalances = useCallback(async () => {
    if (!queryClient || !addressActive) return null;
    const result = await queryClient.getAllBalances(addressActive);
    return reduceBalances(result);
  }, [queryClient, addressActive]);

  const refresh = useCallback(() => {
    setUpdate((item) => item + 1);

    // Snapshot current balance for change detection
    if (allBalances) {
      snapshotRef.current = JSON.stringify(allBalances);
    }

    stopPolling();
    let count = 0;
    pollingRef.current = setInterval(async () => {
      count++;
      const balances = await fetchBalances();
      if (balances && JSON.stringify(balances) !== snapshotRef.current) {
        setAllBalances(balances);
        stopPolling();
        return;
      }
      if (count >= 30) {
        stopPolling();
      }
    }, 10000);
  }, [allBalances, fetchBalances, stopPolling]);

  // Cleanup polling on unmount
  useEffect(() => () => stopPolling(), [stopPolling]);

  useEffect(() => {
    const getAllBalances = async () => {
      if (queryClient && addressActive) {
        const getAllBalancesPromise = await queryClient.getAllBalances(addressActive);
        const balances = reduceBalances(getAllBalancesPromise);
        setAllBalances(balances);
      }
    };
    getAllBalances();
  }, [addressActive, queryClient, _update]);

  useEffect(() => {
    (async () => {
      if (!addressActive) {
        return;
      }

      const vested = {
        [DENOM_LIQUID]: 0,
        millivolt: 0,
        milliampere: 0,
      };

      const getAccount = await authAccounts(addressActive);
      if (
        getAccount.account.vesting_periods &&
        getAccount.account.base_vesting_account.original_vesting
      ) {
        const { vesting_periods: vestingPeriods } = getAccount.account;
        const { original_vesting: originalVestingAmount } = getAccount.account.base_vesting_account;
        const { start_time: startTime } = getAccount.account;
        const reduceOriginalVestingAmount = reduceBalances(originalVestingAmount);
        // eslint-disable-next-line no-restricted-syntax
        for (const key in reduceOriginalVestingAmount) {
          if (Object.hasOwn(reduceOriginalVestingAmount, key)) {
            const element = reduceOriginalVestingAmount[key];
            if (Object.hasOwn(vested, key)) {
              vested[key] += element;
            }
          }
        }

        const vestedAmountReduce = getVestingPeriodsData(vestingPeriods, startTime);
        // eslint-disable-next-line no-restricted-syntax
        for (const key in vestedAmountReduce) {
          if (Object.hasOwn(vestedAmountReduce, key)) {
            const element = vestedAmountReduce[key];
            if (Object.hasOwn(vested, key)) {
              vested[key] -= element;
            }
          }
        }
      }
      setVestedAmount(vested);
    })();
  }, [addressActive]);

  useEffect(() => {
    if (allBalances !== null && vestedAmount !== null) {
      const liquid = { ...allBalances };
      // eslint-disable-next-line no-restricted-syntax
      for (const key in vestedAmount) {
        if (Object.hasOwn(vestedAmount, key)) {
          const elementVestedAmount = vestedAmount[key];
          if (Object.hasOwn(liquid, key)) {
            const liquidAmount = liquid[key] - elementVestedAmount;
            // if (key === 'millivolt' || key === 'milliampere') {
            //   liquidAmount = liquidAmount;
            // }
            liquid[key] = liquidAmount > 0 ? liquidAmount : 0;
          }
        }
      }
      setLiquidBalances(liquid);
    }
  }, [allBalances, vestedAmount]);

  return { liquidBalances, refresh };
}

export default useGetBalances;

import { useEffect } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { getAccountsPassports, selectAccountsPassports } from '../passports.redux';

function useAccountsPassports() {
  const queryClient = useQueryClient();

  const { accounts } = useAppSelector((store) => store.pocket);
  const accountsPassports = useAppSelector(selectAccountsPassports);
  const dispatch = useAppDispatch();

  useEffect(() => {
    if (!queryClient) {
      return;
    }

    dispatch(getAccountsPassports(queryClient));
  }, [queryClient, dispatch]);

  return accountsPassports;
}

export default useAccountsPassports;

import { useEffect } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { getCommunityPassports, selectCommunityPassports } from '../passports.redux';

function useCommunityPassports() {
  const queryClient = useQueryClient();

  const { community } = useAppSelector((store) => store.currentAccount);
  const communityPassports = useAppSelector(selectCommunityPassports);
  const dispatch = useAppDispatch();

  useEffect(() => {
    if (!queryClient) {
      return;
    }

    dispatch(getCommunityPassports(queryClient));
  }, [queryClient, dispatch]);

  return communityPassports;
}

export default useCommunityPassports;

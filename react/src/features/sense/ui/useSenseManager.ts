import { useEffect, useRef } from 'react';
import { useBackend } from 'src/contexts/backend/backend';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { getSenseList, reset } from '../redux/sense.redux';

function useSenseManager() {
  const currentAddress = useAppSelector(selectCurrentAddress);
  const { isLoading: listIsLoading } = useAppSelector((store) => store.sense.list);
  const dispatch = useAppDispatch();

  // TODO: usePrevious hook
  const lastAddress = useRef<string>();
  const { senseApi } = useBackend();

  // refetch sense list when address changes
  useEffect(() => {
    if (!currentAddress || listIsLoading) {
      return;
    }

    // main account changed
    if (lastAddress.current !== currentAddress) {
      if (lastAddress.current) {
        dispatch(reset());
      }

      if (senseApi) {
        dispatch(getSenseList(senseApi));

        lastAddress.current = currentAddress;
      }
    }
  }, [currentAddress, dispatch, senseApi, listIsLoading]);

  return null;
}

export default useSenseManager;

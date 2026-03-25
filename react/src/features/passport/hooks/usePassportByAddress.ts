import { useEffect } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { fromBech32 } from 'src/utils/utils';
import { getPassport } from '../passports.redux';

type Props = {
  address: string | null | undefined;
};

// add 'refresh' prop
function usePassportByAddress(
  address: Props['address'],
  {
    skip,
  }: {
    skip?: boolean;
  } = { skip: false }
) {
  const queryClient = useQueryClient();

  let bostromAddress: string | undefined;
  if (address) {
    if (address.includes('bostrom')) {
      bostromAddress = address;
    } else {
      bostromAddress = fromBech32(address, 'bostrom');
    }
  }

  const currentPassport = useAppSelector((state) =>
    bostromAddress ? state.passports[bostromAddress] : null
  );
  const dispatch = useAppDispatch();

  useEffect(() => {
    if (!queryClient || !bostromAddress || skip || currentPassport?.loading) {
      return;
    }

    if (!currentPassport || currentPassport.data === undefined) {
      dispatch(getPassport({ address: bostromAddress, queryClient }));
    }
  }, [bostromAddress, queryClient, dispatch, currentPassport, skip]);

  const data = currentPassport?.data;

  return {
    // TODO: remove 'passport' prop
    passport: data,
    data,
    loading: currentPassport?.loading || false,
    // error: null,
  };
}

export default usePassportByAddress;

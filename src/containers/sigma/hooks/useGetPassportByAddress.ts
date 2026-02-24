import { PATTERN_CYBER } from 'src/constants/patterns';
import usePassportContract from 'src/features/passport/usePassportContract';
import { Citizenship } from 'src/types/citizenship';

function useGetPassportByAddress(accounts: any) {
  let address =
    accounts?.account?.cyber?.bech32 || accounts?.cyber?.bech32 || accounts?.bech32 || accounts;

  // temp for debug
  if (typeof address === 'object') {
    address = '';
    // debugger;
  }

  const { data, loading, error } = usePassportContract<Citizenship>({
    query: {
      active_passport: {
        address,
      },
    },
    skip: !address || !address.match(PATTERN_CYBER),
  });

  return {
    passport: data,
    loading,
    error,
  };
}

export default useGetPassportByAddress;

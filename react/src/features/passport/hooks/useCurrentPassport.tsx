import { useAppSelector } from 'src/redux/hooks';
import { fromBech32 } from 'src/utils/utils';
import useCurrentAddress from '../../../hooks/useCurrentAddress';

function useCurrentPassport() {
  const currentAddress = useCurrentAddress();

  const bostromAddress = currentAddress && fromBech32(currentAddress, 'bostrom');

  const currentPassport = useAppSelector((store) =>
    bostromAddress ? store.passports[bostromAddress] : null
  );
  return currentPassport;
}

export default useCurrentPassport;

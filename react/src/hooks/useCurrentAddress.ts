import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppSelector } from 'src/redux/hooks';

function useCurrentAddress() {
  const currentAddress = useAppSelector(selectCurrentAddress);
  return currentAddress;
}

export default useCurrentAddress;

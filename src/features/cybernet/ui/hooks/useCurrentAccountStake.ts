import useCybernetContract from 'src/features/cybernet/ui/useQueryCybernetContract.refactor';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppSelector } from 'src/redux/hooks';
import { StakeInfo } from '../../types';

type Props = {
  address: string;
  contractAddress?: string;
  skip?: boolean;
};

export function useStake({ address, contractAddress, skip }: Props) {
  const query = useCybernetContract<StakeInfo>({
    query: {
      get_stake_info_for_coldkey: {
        coldkey: address,
      },
    },
    contractAddress,
    skip: !address || skip,
  });

  return query;
}

function useCurrentAccountStake({ skip } = {}) {
  const currentAddress = useAppSelector(selectCurrentAddress);

  return useStake({ address: currentAddress, skip });
}

export default useCurrentAccountStake;

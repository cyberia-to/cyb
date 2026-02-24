import useExecuteContractWithWaitAndAdviser, {
  Props as ExecuteContractProps,
} from '../../../hooks/contract/useExecuteContractWithWaitAndAdviser';
import { useCybernet } from './cybernet.context';

type Props = Omit<ExecuteContractProps, 'contractAddress'>;

function useExecuteCybernetContract({ query, funds, onSuccess, successMessage }: Props) {
  const { selectedContract } = useCybernet();

  return useExecuteContractWithWaitAndAdviser({
    contractAddress: selectedContract.address,
    query,
    funds,
    onSuccess,
    successMessage,
  });
}

export default useExecuteCybernetContract;

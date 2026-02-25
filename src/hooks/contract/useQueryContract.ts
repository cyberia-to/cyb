import useQueryClientMethod from '../useQueryClientMethod';

function useQueryContract(address: string, query: any) {
  const result = useQueryClientMethod('queryContractSmart', [address, query]);
  return result;
}

export default useQueryContract;

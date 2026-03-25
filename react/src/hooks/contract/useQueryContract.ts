import useQueryClientMethod from '../useQueryClientMethod';

function useQueryContract(
  address: string,
  query: any,
  options?: { refetchInterval?: number | false }
) {
  const result = useQueryClientMethod('queryContractSmart', [address, query], options);
  return result;
}

export default useQueryContract;

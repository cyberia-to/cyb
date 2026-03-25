import { CyberClient } from '@cybercongress/cyber-js';

// use this for type somehow https://github.com/cybercongress/cybernet/tree/main/schema/raw
export type CybernetContractQuery = object;

export function queryCybernetContract(
  address: string,
  query: CybernetContractQuery,
  queryClient: CyberClient
) {
  return queryClient.queryContractSmart(address, query);
}

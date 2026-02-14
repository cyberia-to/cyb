import { createPagination } from '@cosmjs/stargate';
import { cyber } from '@cybercongress/cyber-ts';
import { DelegationResponse } from 'cosmjs-types/cosmos/staking/v1beta1/staking';

// eslint-disable-next-line import/prefer-default-export
export const getDelegatorDelegations = async (
  client: Awaited<ReturnType<typeof cyber.ClientFactory.createRPCQueryClient>>,
  addressBech32: string
): Promise<DelegationResponse[]> => {
  let nextKey: Uint8Array | undefined;
  const delegationData: DelegationResponse[] = [];

  let done = false;
  while (!done) {
    const responsedelegatorDelegations =
      // eslint-disable-next-line no-await-in-loop
      await client.cosmos.staking.v1beta1.delegatorDelegations({
        delegatorAddr: addressBech32,
        pagination: createPagination(nextKey),
      });

    delegationData.push(...responsedelegatorDelegations.delegationResponses);

    const key = responsedelegatorDelegations?.pagination?.nextKey;

    if (key?.byteLength) {
      nextKey = key;
    } else {
      done = true;
    }
  }

  return delegationData;
};

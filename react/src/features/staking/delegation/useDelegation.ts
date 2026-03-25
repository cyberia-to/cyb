import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppSelector } from 'src/redux/hooks';
import useQueryClientMethod from '../../../hooks/useQueryClientMethod';

const methodName = 'delegation';

type MethodParams = NonNullable<Parameters<typeof useQueryClientMethod<typeof methodName>>[1]>;

type Params = MethodParams;

function useDelegationFunc(...[delegatorAddress, validatorAddress]: Params) {
  const { data, ...rest } = useQueryClientMethod<typeof methodName>(methodName, [
    delegatorAddress,
    validatorAddress,
  ]);

  const { delegationResponse } = data || {};

  return {
    data: delegationResponse
      ? {
          validatorAddress: delegationResponse?.delegation?.validatorAddress,
          shares: delegationResponse?.delegation?.shares,
          balance: delegationResponse?.balance,
        }
      : undefined,
    ...rest,
  };
}

// TODO: add skip somewhere
function useDelegation(validatorAddress: string) {
  const address = useAppSelector(selectCurrentAddress);

  return useDelegationFunc(address, validatorAddress);
}

export default useDelegation;

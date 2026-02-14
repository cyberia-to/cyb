import { useCyberClient } from 'src/contexts/queryCyberClient';

function useValidatorDelegations(operatorAddress?: string) {
  const { hooks } = useCyberClient();

  const { data: dataValidatorDelegations } =
    hooks.cosmos.staking.v1beta1.useValidatorDelegations({
      request: { validatorAddr: operatorAddress || '' },
      options: {
        enabled: Boolean(hooks && operatorAddress),
      },
    });

  const delegations = dataValidatorDelegations
    ? BigInt(dataValidatorDelegations.pagination?.total || 0).toString()
    : undefined;

  return { delegations, dataValidatorDelegations };
}

export default useValidatorDelegations;

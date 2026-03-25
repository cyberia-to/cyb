import { useCyberClient } from 'src/contexts/queryCyberClient';
import { useSphereContext } from 'src/pages/Sphere/Sphere.context';

function useValidatorByContext(operatorAddress?: string) {
  const { hooks } = useCyberClient();

  const { validators } = useSphereContext();

  const valInfo = operatorAddress
    ? validators.find((item) => item.operatorAddress === operatorAddress)
    : undefined;

  const { data: valRes } = hooks.cosmos.staking.v1beta1.useValidator({
    request: { validatorAddr: operatorAddress || '' },
    options: { enabled: Boolean(operatorAddress && !valInfo) },
  });

  const validatorInfo = valInfo || (valRes ? valRes.validator : undefined);

  return { validatorInfo };
}

export default useValidatorByContext;

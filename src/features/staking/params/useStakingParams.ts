import useQueryClientMethod from '../../../hooks/useQueryClientMethod';

function useStakingParams() {
  return useQueryClientMethod<'stakingParams'>('stakingParams');
}

export default useStakingParams;

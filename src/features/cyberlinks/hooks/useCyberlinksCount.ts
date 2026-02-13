import { useCyberlinksCountByParticleQuery } from 'src/generated/graphql';

const getVar = (type: 'from' | 'to', cid: string, neuron) => {
  return { [`particle_${type}`]: { _eq: cid }, neuron: { _eq: neuron } };
};

function useCyberlinksCount(cid: string, neuron?: string) {
  const toCountQuery = useCyberlinksCountByParticleQuery({
    variables: { where: getVar('to', cid, neuron) },
  });
  const fromCountQuery = useCyberlinksCountByParticleQuery({
    variables: { where: getVar('from', cid, neuron) },
  });

  return {
    data: {
      to: toCountQuery.data?.cyberlinks_aggregate?.aggregate?.count as
        | number
        | undefined,
      from: fromCountQuery.data?.cyberlinks_aggregate?.aggregate?.count as
        | number
        | undefined,
    },
    loading: toCountQuery.loading || fromCountQuery.loading,
    error: toCountQuery.error || fromCountQuery.error,
  };
}

export default useCyberlinksCount;

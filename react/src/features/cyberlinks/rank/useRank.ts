import useQueryClientMethod from 'src/hooks/useQueryClientMethod';

function useRank(cid: string) {
  const { data } = useQueryClientMethod('rank', [cid]);

  const rank = data?.rank;

  return rank ? Number(rank) : undefined;
}

export default useRank;

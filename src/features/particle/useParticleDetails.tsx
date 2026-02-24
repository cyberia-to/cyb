import { useQuery } from '@tanstack/react-query';
import useQueueIpfsContent from 'src/hooks/useQueueIpfsContent';

/**
 @deprecated TODO: refactor to useParticle hook
  */
function useParticleDetails(cid: string, { skip = false } = {}, parentId?: string) {
  const { fetchWithDetails } = useQueueIpfsContent(parentId || cid);

  const { data, isLoading, error } = useQuery(
    ['particleDetails', cid],
    async () => {
      // FIXME: temporary solution
      return fetchWithDetails!(cid) || null;
    },
    {
      enabled: Boolean(cid && fetchWithDetails) && !skip,
    }
  );

  return {
    data,
    loading: isLoading,
    error,
  };
}

export default useParticleDetails;

import { useQuery } from '@tanstack/react-query';
import { getIpfsHash } from 'src/utils/ipfs/helpers';

function useGetIPFSHash(content: any) {
  const { data } = useQuery(
    ['ipfsHash', content],
    () => {
      return getIpfsHash(content);
    },
    {
      enabled: Boolean(content),
    }
  );

  return data;
}

export default useGetIPFSHash;

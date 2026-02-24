import { useEffect, useState } from 'react';
import useQueueIpfsContent from 'src/hooks/useQueueIpfsContent';
import { CommunityDto } from 'src/services/CozoDb/types/dto';
import { fetchCommunity } from 'src/services/community/community';
import { NeuronAddress, ParticleCid } from 'src/types/base';
import { makeCancellable } from 'src/utils/async/promise';
import { removeDublicates } from 'src/utils/list';

function useGetCommunity(address: string | null, { skip }: { skip?: boolean }) {
  const { fetchParticleAsync } = useQueueIpfsContent();
  const [abortController, setAbortController] = useState(new AbortController());

  const [community, setCommunity] = useState({
    following: [] as NeuronAddress[],
    followers: [] as NeuronAddress[],
    friends: [] as NeuronAddress[],
  });

  // TODO: maybe refactor
  const [loading, _setLoading] = useState({
    following: false,
    followers: false,
    friends: false,
  });

  const [isLoaded, setIsLoaded] = useState(false);

  useEffect(() => {
    setCommunity({
      following: [],
      followers: [],
      friends: [],
    });
    abortController.abort();

    setAbortController(abortController);

    setIsLoaded(false);

    // don't set abortController
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [abortController]);

  useEffect(() => {
    if (!address || skip || !fetchParticleAsync || isLoaded) {
      return;
    }

    (async () => {
      const communityRaw: Map<ParticleCid, CommunityDto> = new Map();

      const onResolve = (communityResolved: CommunityDto[]) => {
        communityResolved.forEach((item) => communityRaw.set(item.neuron, item));
        // TODO: exclude dublicates when followe and following from 2 sources
        // const allItems = [...state.community.raw, ...items];

        setCommunity({
          followers: removeDublicates(
            [...communityRaw.values()]
              .filter((item) => item.follower && !item.following)
              .map((item) => item.neuron)
          ),
          following: removeDublicates(
            [...communityRaw.values()]
              .filter((item) => item.following && !item.follower)
              .map((item) => item.neuron)
          ),
          friends: removeDublicates(
            [...communityRaw.values()]
              .filter((item) => item.follower && item.following)
              .map((item) => item.neuron)
          ),
        });
      };

      await makeCancellable(fetchCommunity, abortController.signal)(
        address,
        fetchParticleAsync,
        onResolve
      )
        .catch(() => console.log(`>>>!!! sync community ${address} was cancelled`))
        .finally(() => {
          setIsLoaded(true);
        });

      // TODO: refactor, loading disabled
      // setLoading({ followers: true, following: true, friends: true });
    })();
  }, [isLoaded, fetchParticleAsync, address, skip, abortController]);

  return {
    community,
    communityLoaded: isLoaded,
    loading,
  };
}

export default useGetCommunity;

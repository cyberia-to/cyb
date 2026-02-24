import { useQuery } from '@tanstack/react-query';
import { getFromLink, getToLink } from 'src/services/transactions/lcd';

const reduceParticleArr = (data) => {
  return data.reduce((acc, item) => {
    let addressCreator = '';
    if (item.body.messages[0].neuron) {
      addressCreator = item.body.messages[0].neuron;
    }
    if (item.body.messages[0].sender) {
      addressCreator = item.body.messages[0].sender;
    }
    return [...acc, addressCreator];
  }, []);
};

const offset = '0';
const limit = '20';

const fetchFunc = async (hash, func) => {
  try {
    const responseSearchResults = await func(hash, offset, limit);
    return responseSearchResults.txs || [];
  } catch (_error) {
    return [];
  }
};

function useGetCommunity(cid: string) {
  const { data } = useQuery(
    ['useGetCommunity', cid],
    async () => {
      const responseTo = await fetchFunc(cid, getToLink);
      const responseFrom = await fetchFunc(cid, getFromLink);
      const reduceTo = reduceParticleArr(responseTo);
      const reduceFrom = reduceParticleArr(responseFrom);

      return [...new Set(reduceTo.concat(reduceFrom))];
    },
    {
      enabled: Boolean(cid),
    }
  );
  return { community: data };
}

export default useGetCommunity;

import { useQuery } from '@tanstack/react-query';
import { getTransactions } from 'src/services/transactions/lcd';
import { Option } from 'src/types';
import { CreatorCyberLink } from 'src/types/cyberLink';

// TODO: refactor this
const getCreator = async (cid) => {
  try {
    const response = await getTransactions({
      events: [
        {
          key: 'cyberlink.particleTo',
          value: cid,
        },
      ],
      pagination: {
        limit: 1,
        offset: 0,
      },
    });

    const response2 = await getTransactions({
      events: [
        {
          key: 'cyberlink.particleFrom',
          value: cid,
        },
      ],
      pagination: {
        limit: 1,
        offset: 0,
      },
    });

    const h1 = Number(response.txResponses?.[0]?.height || 0);
    const h2 = Number(response2.txResponses?.[0]?.height || 0);

    if (h1 === 0) {
      return response2;
    }
    if (h2 === 0) {
      return response;
    }

    return h1 < h2 ? response : response2;
  } catch (error) {
    console.log(error);
    return null;
  }
};

function useGetCreator(cid: string) {
  const { data } = useQuery(
    ['useGetCreator', cid],
    async () => {
      return getCreator(cid);
    },
    {
      enabled: Boolean(cid),
    }
  );

  let creator: Option<CreatorCyberLink>;
  if (data?.txResponses?.length > 0) {
    const { txResponses } = data;

    const { neuron, sender } = txResponses[0].tx.body.messages[0];
    const addressCreator = sender || neuron;

    const [{ timestamp }] = txResponses;

    creator = {
      address: addressCreator,
      timestamp,
    };
  }

  return {
    creator,
  };
}

export default useGetCreator;

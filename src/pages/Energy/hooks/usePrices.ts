import { PriceHash } from '@osmonauts/math/types';
import { useQuery } from '@tanstack/react-query';

const getPriceHash = async () => {
  let prices = [];

  try {
    const response = await fetch(
      'https://api-osmosis.imperator.co/tokens/v2/all'
    );
    if (!response.ok) {
      throw Error('Get price error');
    }
    prices = await response.json();
  } catch (err) {
    console.error(err);
  }

  const priceHash = prices.reduce(
    (prev: any, cur: { denom: any; price: any }) => ({
      ...prev,
      [cur.denom]: cur.price,
    }),
    {}
  );

  return priceHash;
};

function usePrices() {
  const { data, refetch } = useQuery(['osmosis', 'priceHash'], async () => {
    return getPriceHash() as Promise<PriceHash>;
  });

  return { data: data || {}, refetch };
}

export default usePrices;

import { useQuery } from '@tanstack/react-query';
import BigNumber from 'bignumber.js';
import { useEffect, useState } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';
import { getNowUtcTime } from 'src/utils/utils';

const keyQuery = 'graphStats';

function useGetGraphStats(refetchInterval: number | undefined) {
  const queryClient = useQueryClient();
  const [changeTimeAmount, setChangeTimeAmount] = useState({
    particles: 0,
    cyberlinks: 0,
    bits: 0,
    beta: 0,
    time: 0,
  });

  const { data, status } = useQuery({
    queryKey: [keyQuery],
    queryFn: async () => {
      let response = {
        particles: 0,
        cyberlinks: 0,
        bits: 0,
        beta: 0,
        timestamp: '',
      };
      const responseGraphStats = await queryClient.graphStats();

      if (responseGraphStats !== null) {
        const { cyberlinks, particles } = responseGraphStats;
        const bits = new BigNumber(40)
          .multipliedBy(cyberlinks)
          .plus(new BigNumber(40).multipliedBy(particles))
          .toNumber();
        let beta = 0;
        if (new BigNumber(cyberlinks).comparedTo(0) && new BigNumber(particles).comparedTo(0)) {
          beta = new BigNumber(cyberlinks)
            .dividedBy(particles)
            .dp(3, BigNumber.ROUND_FLOOR)
            .toNumber();
        }
        const d = getNowUtcTime();
        response = {
          cyberlinks,
          particles,
          bits,
          beta,
          timestamp: d,
        };
      }
      return response;
    },
    enabled: Boolean(queryClient),
    refetchInterval,
  });

  useEffect(() => {
    if (data && data !== null) {
      const lastgraphStatsLs = localStorage.getItem(keyQuery);
      if (lastgraphStatsLs !== null) {
        const oldData = JSON.parse(lastgraphStatsLs);

        const timeChange = new BigNumber(data.timestamp).minus(oldData.timestamp).toNumber();
        const cyberlinksAmount = new BigNumber(data.cyberlinks)
          .minus(oldData.cyberlinks)
          .toNumber();
        const particlesAmount = new BigNumber(data.particles).minus(oldData.particles).toNumber();
        const bitsAmount = new BigNumber(data.bits).minus(oldData.bits).toNumber();
        const betaAmount = new BigNumber(data.beta).minus(oldData.beta).toNumber();

        if (timeChange > 0) {
          setChangeTimeAmount({
            cyberlinks: cyberlinksAmount,
            particles: particlesAmount,
            bits: bitsAmount,
            beta: betaAmount,
            time: timeChange,
          });
        }
      }
      localStorage.setItem(keyQuery, JSON.stringify(data));
    }
  }, [data]);

  return { data, changeTimeAmount, status };
}

export default useGetGraphStats;

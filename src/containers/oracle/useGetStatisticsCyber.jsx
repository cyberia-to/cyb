import { useEffect, useState } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';

function useGetStatisticsCyber() {
  const queryClient = useQueryClient();
  const [knowledge, setKnowledge] = useState({
    linksCount: 0,
    cidsCount: 0,
    accountsCount: 0,
    inlfation: 0,
    stakedCyb: 0,
    activeValidatorsCount: 0,
  });

  useEffect(() => {
    const getStatisticsBrain = async () => {
      if (queryClient) {
        const totalCyb = {};
        let stakedCyb = 0;

        const responseGraphStats = await queryClient.graphStats();
        const { particles, cyberlinks } = responseGraphStats;
        setKnowledge((item) => ({
          ...item,
          linksCount: cyberlinks,
          cidsCount: particles,
        }));

        const responseHeroesActive = await queryClient.validators('BOND_STATUS_BONDED');
        const { validators } = responseHeroesActive;
        setKnowledge((item) => ({
          ...item,
          activeValidatorsCount: validators.length,
        }));

        const datagetTotalSupply = await queryClient.totalSupply();
        if (Object.keys(datagetTotalSupply).length > 0) {
          datagetTotalSupply.forEach((item) => {
            totalCyb[item.denom] = parseFloat(item.amount);
          });
        }

        if (totalCyb.boot && totalCyb.sboot) {
          const { boot, sboot } = totalCyb;
          stakedCyb = sboot / boot;
        }
        setKnowledge((item) => ({
          ...item,
          stakedCyb,
        }));
      }
    };
    getStatisticsBrain();
  }, [queryClient]);

  return {
    knowledge,
  };
}

export default useGetStatisticsCyber;

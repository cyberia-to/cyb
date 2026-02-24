import BigNumber from 'bignumber.js';
import { useEffect, useMemo, useState } from 'react';
import { BASE_DENOM, DENOM_LIQUID } from 'src/constants/config';
import { useQueryClient } from 'src/contexts/queryClient';
import { useCyberClient } from 'src/contexts/queryCyberClient';

import { getProposals } from '../../../utils/governance';
import { coinDecimals, covertUint8ArrayToString } from '../../../utils/utils';

function useGetStatisticsCyber() {
  const queryClient = useQueryClient();
  const [knowledge, setKnowledge] = useState({
    linksCount: 0,
    cidsCount: 0,
    accountsCount: 0,
    staked: 0,
    activeValidatorsCount: 0,
  });
  const [government, setGovernment] = useState({
    proposals: 0,
    communityPool: 0,
  });

  const { hooks } = useCyberClient();

  const { data: inflationData } = hooks.cosmos.mint.v1beta1.useInflation({
    request: {},
  });

  useEffect(() => {
    const getGovernment = async () => {
      if (queryClient) {
        let communityPool = 0;
        let proposals = 0;

        const dataCommunityPool = await queryClient.communityPool();
        const { pool } = dataCommunityPool;
        if (dataCommunityPool !== null) {
          communityPool = Math.floor(coinDecimals(parseFloat(pool[0].amount)));
        }

        const dataProposals = await getProposals();
        if (dataProposals !== null && dataProposals.length > 0) {
          proposals = dataProposals.length + 1;
        }

        setGovernment({ proposals, communityPool });
      }
    };
    getGovernment();
  }, [queryClient]);

  useEffect(() => {
    const getStatisticsBrain = async () => {
      if (queryClient) {
        const totalCyb = {};
        let staked = 0;

        const responseGraphStats = await queryClient.graphStats();
        const { cyberlinks, particles } = responseGraphStats;
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

        if (totalCyb[BASE_DENOM] && totalCyb[DENOM_LIQUID]) {
          staked = new BigNumber(totalCyb[DENOM_LIQUID])
            .dividedBy(totalCyb[BASE_DENOM])
            .toString(10);
        }
        setKnowledge((item) => ({
          ...item,
          staked,
        }));
      }
    };
    getStatisticsBrain();
  }, [queryClient]);

  const inflation = useMemo(() => {
    if (inflationData) {
      return (
        // add 10 ** 18 to variable
        BigNumber(covertUint8ArrayToString(inflationData.inflation)) / 10 ** 18
      );
    }
    return;
  }, [inflationData]);

  return {
    ...government,
    ...knowledge,
    inlfation: inflation,
  };
}

export default useGetStatisticsCyber;

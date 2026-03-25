import BigNumber from 'bignumber.js';
// import { QueryTallyResultResponse } from 'cosmjs-types/cosmos/gov/v1beta1/query';
import { useMemo } from 'react';
import { useCyberClient } from 'src/contexts/queryCyberClient';

const finalTallyResult = (item) => {
  const finalVotes = {
    yes: 0,
    no: 0,
    abstain: 0,
    noWithVeto: 0,
    finalTotalVotes: 0,
  };
  let finalTotalVotes = 0;
  const yes = parseInt(item.yesCount, 10);
  const abstain = parseInt(item.abstainCount, 10);
  const no = parseInt(item.noCount, 10);
  const noWithVeto = parseInt(item.noWithVetoCount, 10);

  finalTotalVotes = yes + abstain + no + noWithVeto;
  if (finalTotalVotes !== 0) {
    finalVotes.yes = (yes / finalTotalVotes) * 100;
    finalVotes.no = (no / finalTotalVotes) * 100;
    finalVotes.abstain = (abstain / finalTotalVotes) * 100;
    finalVotes.noWithVeto = (noWithVeto / finalTotalVotes) * 100;
    finalVotes.finalTotalVotes = finalTotalVotes;
  }

  return finalVotes;
};

function useTallyResult(id?: string) {
  const { hooks } = useCyberClient();

  const { data: tallyResponse, refetch } = hooks.cosmos.gov.v1.useTallyResult({
    request: { proposalId: id },
    options: { enabled: Boolean(id) },
  });
  const { data: stakingPool } = hooks.cosmos.staking.v1beta1.usePool({
    request: {},
  });
  const { data: tallyingResponse } = hooks.cosmos.gov.v1.useParams({
    request: { paramsType: 'tallying' },
  });

  const tallyResultData = useMemo(() => {
    let tally = {
      participation: 0,
      yes: 0,
      abstain: 0,
      no: 0,
      noWithVeto: 0,
    };

    if (!tallyResponse || !stakingPool) {
      return tally;
    }

    const tallyTemp = finalTallyResult(tallyResponse.tally);

    const participation = new BigNumber(tallyTemp.finalTotalVotes)
      .dividedBy(stakingPool.pool.bondedTokens)
      .multipliedBy(100)
      .toNumber();

    tally = { ...tally, ...tallyTemp, participation };

    return tally;
  }, [tallyResponse, stakingPool]);

  const tallyingData = useMemo(() => {
    let tallying = { quorum: '0', threshold: '0', vetoThreshold: '0' };

    if (tallyingResponse) {
      tallying = { ...tallying, ...tallyingResponse.tallyParams };
    }

    return tallying;
  }, [tallyingResponse]);

  return {
    refetch,
    tallyResult: tallyResultData,
    tallying: tallyingData,
  };
}

export default useTallyResult;

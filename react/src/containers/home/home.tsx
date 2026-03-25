import { Pane } from '@cybercongress/gravity';
import BigNumber from 'bignumber.js';
import { useEffect, useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { BASE_DENOM } from 'src/constants/config';
import { useAppData } from 'src/contexts/appData';
import { useQueryClient } from 'src/contexts/queryClient';
import { useContractsCountQuery } from 'src/generated/graphql';
import { CardStatisics, Dots, MainContainer } from '../../components';
import { formatCurrency, formatNumber } from '../../utils/utils';
import useGetStatisticsCyber from '../brain/hooks/getStatisticsCyber';
import KnowledgeTab from '../brain/tabs/knowledge';
import Txs from '../brain/tx';
import { getNumTokens, getStateGift } from '../portal/utils';
import { useGetNegentropy } from '../temple/hooks';

const PREFIXES = [
  {
    prefix: 'T',
    power: 1024 * 10 ** 9,
  },
  {
    prefix: 'G',
    power: 1024 * 10 ** 6,
  },
  {
    prefix: 'M',
    power: 1024 * 10 ** 3,
  },
  {
    prefix: 'K',
    power: 1024,
  },
];

function ContainerGrid({ children }) {
  return (
    <Pane
      marginTop={10}
      marginBottom={50}
      display="grid"
      gridTemplateColumns="repeat(auto-fit, minmax(210px, 1fr))"
      gridGap="20px"
    >
      {children}
    </Pane>
  );
}

function Home() {
  const { block } = useAppData();
  const queryClient = useQueryClient();
  const [memory, setMemory] = useState(0);
  const [memoryLoader, setMemoryLoader] = useState(true);
  const [counCitizenshipst, setCounCitizenshipst] = useState(0);
  const [citizensClaim, setCitizensClaim] = useState(0);
  const { loading, data } = useContractsCountQuery();
  const {
    linksCount,
    cidsCount,
    accountsCount,
    inlfation,
    staked,
    activeValidatorsCount,
    communityPool,
    proposals,
  } = useGetStatisticsCyber();
  const { data: negentropy, status } = useGetNegentropy(undefined);

  useEffect(() => {
    const cheeckStateRelease = async () => {
      // setLoading(true);
      if (queryClient) {
        try {
          const queryResponseResultState = await getStateGift(queryClient);
          const respnseNumTokens = await getNumTokens(queryClient);
          if (respnseNumTokens?.count) {
            setCounCitizenshipst(parseFloat(respnseNumTokens.count));
          }

          if (queryResponseResultState?.claims) {
            const { claims } = queryResponseResultState;
            setCitizensClaim(claims);
          }
        } catch (error) {
          console.log('error', error);
        }
      }
    };
    cheeckStateRelease();
  }, [queryClient]);

  useEffect(() => {
    try {
      const getGraphStats = async () => {
        setMemoryLoader(true);
        if (queryClient) {
          const responseGraphStats = await queryClient.graphStats();
          const { particles, cyberlinks } = responseGraphStats;
          const bits = 40 * parseFloat(cyberlinks) + 40 * parseFloat(particles);
          setMemory(bits);
        }
        setMemoryLoader(false);
      };
      getGraphStats();
    } catch (e) {
      console.log(e);
      setMemory(0);
      setMemoryLoader(false);
    }
  }, [queryClient]);

  const useGetBeta = useMemo(() => {
    const link = new BigNumber(linksCount);
    const particles = new BigNumber(cidsCount);

    if (link.comparedTo(0) && particles.comparedTo(0)) {
      const beta = link.dividedBy(particles).dp(3, BigNumber.ROUND_FLOOR).toNumber();
      return beta;
    }

    return 0;
  }, [linksCount, cidsCount]);

  return (
    <MainContainer>
      <ContainerGrid>
        <CardStatisics
          value={status === 'loading' ? <Dots /> : `${negentropy?.negentropy || 0} bits`}
          title="Negentropy"
          styleContainer={{ minWidth: 'unset' }}
        />
        <CardStatisics
          value={memoryLoader ? <Dots /> : formatCurrency(memory, 'B', 0, PREFIXES)}
          title="GPU memory"
          styleContainer={{ minWidth: 'unset' }}
        />
        <Link to="/network/bostrom/tx">
          <CardStatisics
            title="Transactions"
            value={<Txs />}
            styleContainer={{ minWidth: 'unset' }}
            link
          />
        </Link>
        <Link to="/network/bostrom/blocks">
          <CardStatisics
            title="Blocks"
            value={formatNumber(parseFloat(block))}
            styleContainer={{ minWidth: 'unset' }}
            link
          />
        </Link>
      </ContainerGrid>
      <ContainerGrid>
        <KnowledgeTab
          linksCount={parseInt(linksCount, 10)}
          cidsCount={parseInt(cidsCount, 10)}
          accountsCount={parseInt(accountsCount, 10)}
          inlfation={parseFloat(inlfation)}
        />
      </ContainerGrid>
      <ContainerGrid>
        <CardStatisics
          value={`${formatNumber(staked * 100, 2)} %`}
          title={`Staked ${BASE_DENOM.toUpperCase()}`}
          // styleContainer={{ minWidth: 'unset' }}
        />
        <CardStatisics
          value={formatNumber(activeValidatorsCount)}
          title="Active heroes"
          // styleContainer={{ minWidth: 'unset' }}
        />
        <CardStatisics
          value={formatNumber(communityPool)}
          title="Community pool"
          // styleContainer={{ minWidth: 'unset' }}
        />
        <CardStatisics
          value={formatNumber(proposals)}
          title="Proposals"
          // styleContainer={{ minWidth: 'unset' }}
        />
      </ContainerGrid>
      <ContainerGrid>
        <CardStatisics
          value={formatNumber(counCitizenshipst)}
          title="Citizens"
          // styleContainer={{ minWidth: 'unset' }}
        />
        <CardStatisics
          value={formatNumber(citizensClaim)}
          title="Gift claims"
          // styleContainer={{ minWidth: 'unset' }}
        />
        <CardStatisics
          title="Contracts"
          value={loading ? <Dots /> : formatNumber(data?.contracts_aggregate.aggregate?.count || 0)}
        />
        <CardStatisics
          value={formatNumber(useGetBeta)}
          title="Beta"
          // styleContainer={{ minWidth: 'unset' }}
        />
      </ContainerGrid>
    </MainContainer>
  );
}

export default Home;

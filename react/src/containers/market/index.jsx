/* eslint-disable no-await-in-loop */

import { Pane, Text } from '@cybercongress/gravity';
import { useEffect, useState } from 'react';
import { connect } from 'react-redux';
import { Link, useParams } from 'react-router-dom';
import { useDevice } from 'src/contexts/device';
import { useQueryClient } from 'src/contexts/queryClient';
import { enqueueLinksSave } from 'src/services/backend/channels/BackendQueueChannel/backendQueueSenders';
import { mapLinkToLinkDto } from 'src/services/CozoDb/mapping';
import { getIpfsHash } from 'src/utils/ipfs/helpers';
import { Loading, MainContainer } from '../../components';
import { getRankGrade, searchByHash } from '../../utils/search/utils';
import { coinDecimals } from '../../utils/utils';
import ActionBarCont from './actionBarContainer';
import InfoTokens from './infoTokens';
import SearchTokenInfo from './searchTokensInfo';
import useGetCybernomics from './useGetTokensInfo';
import useSetActiveAddress from './useSetActiveAddress';

function ContainerGrid({ children }) {
  return (
    <Pane
      marginTop={10}
      marginBottom={50}
      display="grid"
      gridTemplateColumns="repeat(auto-fit, minmax(250px, 1fr))"
      gridGap="20px"
    >
      {children}
    </Pane>
  );
}

const reduceSearchResults = (data, query) => {
  return data.reduce(
    (obj, item) => ({
      ...obj,
      [item.particle]: {
        particle: item.particle,
        rank: coinDecimals(item.rank),
        grade: getRankGrade(coinDecimals(item.rank)),
        status: 'impossibleLoad',
        query,
        text: item.particle,
        content: false,
      },
    }),
    {}
  );
};

function Market({ defaultAccount }) {
  const { addressActive } = useSetActiveAddress(defaultAccount);
  const queryClient = useQueryClient();

  const { tab = 'BOOT' } = useParams();
  const { gol, cyb, boot, hydrogen, milliampere, millivolt, tocyb } = useGetCybernomics();
  const [resultSearch, setResultSearch] = useState([]);
  const [loadingSearch, setLoadingSearch] = useState(true);
  const [keywordHash, setKeywordHash] = useState('');
  const [update, setUpdate] = useState(0);
  const [rankLink, setRankLink] = useState(null);
  const [page, setPage] = useState(0);
  const [allPage, setAllPage] = useState(0);
  const { isMobile: mobile } = useDevice();

  useEffect(() => {
    const getFirstItem = async () => {
      if (queryClient) {
        setPage(0);
        setAllPage(0);
        setResultSearch([]);
        setLoadingSearch(true);
        const hash = await getIpfsHash(tab);
        setKeywordHash(hash);
        const response = await searchByHash(queryClient, hash, 0);
        if (response.result && response.result.length > 0) {
          const dataApps = reduceSearchResults(response.result, tab);
          setResultSearch(dataApps);
          setLoadingSearch(false);
          setAllPage(Math.ceil(parseFloat(response.pagination.total) / 10));
          setPage((item) => item + 1);

          enqueueLinksSave(response.result.map((l) => mapLinkToLinkDto(hash, l.particle)));
        } else {
          setResultSearch([]);
          setLoadingSearch(false);
        }
      } else {
        setResultSearch([]);
        setLoadingSearch(false);
      }
    };
    getFirstItem();
  }, [queryClient, tab]);

  const fetchMoreData = async () => {
    // a fake async api call like which sends
    // 20 more records in 1.5 secs
    let links = [];
    const response = await searchByHash(queryClient, keywordHash, page);
    if (response.result) {
      links = reduceSearchResults(response, tab);

      enqueueLinksSave(response.result.map((l) => mapLinkToLinkDto(keywordHash, l.particle)));
    }

    setTimeout(() => {
      setResultSearch((itemState) => ({ ...itemState, ...links }));
      setPage((itemPage) => itemPage + 1);
    }, 500);
  };

  useEffect(() => {
    setRankLink(null);
  }, []);

  const onClickRank = async (key) => {
    if (rankLink === key) {
      setRankLink(null);
    } else {
      setRankLink(key);
    }
  };

  return (
    <>
      <MainContainer>
        {addressActive === null && (
          <Pane boxShadow="0px 0px 5px #36d6ae" paddingX={20} paddingY={20} marginY={20}>
            <Text fontSize="16px" color="#fff">
              {/* eslint-disable-next-line react/no-unescaped-entities */}
              Subscribe to someone to make your feed work. Until then, we'll show you the project
              feed. Start by adding a ledger to <Link to="/">your pocket</Link>.
            </Text>
          </Pane>
        )}
        <ContainerGrid>
          {tab === 'BOOT' && <InfoTokens selectedTokens={tab} data={boot} />}

          {tab === 'H' && <InfoTokens selectedTokens={tab} data={hydrogen} />}

          {tab === 'A' && <InfoTokens selectedTokens={tab} data={milliampere} />}

          {tab === 'V' && <InfoTokens selectedTokens={tab} data={millivolt} />}

          {tab === 'TOCYB' && <InfoTokens selectedTokens={tab} data={tocyb} />}

          {tab === 'GOL' && (
            <InfoTokens
              selectedTokens={tab}
              data={gol}
              titlePrice="Uniswap price of GGOL in ETH"
              linkSupply="https://etherscan.io/token/0xF4ecdBa8ba4144Ff3a2d8792Cad9051431Aa4F64"
              linkPrice="https://app.uniswap.org/#/swap?inputCurrency=ETH&outputCurrency=0xF4ecdBa8ba4144Ff3a2d8792Cad9051431Aa4F64"
            />
          )}

          {tab === 'CYB' && (
            <InfoTokens selectedTokens={tab} data={cyb} titlePrice="Port price of GCYB in ETH" />
          )}
        </ContainerGrid>
        <ContainerGrid>
          {loadingSearch ? (
            <div
              style={{
                width: '100%',
                display: 'flex',
                justifyContent: 'center',
                alignItems: 'center',
                flexDirection: 'column',
              }}
            >
              <Loading />
              <div style={{ color: '#fff', marginTop: 20, fontSize: 20 }}>Searching</div>
            </div>
          ) : (
            <SearchTokenInfo
              data={resultSearch}
              mobile={mobile}
              selectedTokens={tab}
              onClickRank={onClickRank}
              fetchMoreData={fetchMoreData}
              page={page}
              allPage={allPage}
              parentId={`market.${tab}`}
            />
          )}
        </ContainerGrid>
      </MainContainer>
      {!mobile && (
        <ActionBarCont
          addressActive={addressActive}
          mobile={mobile}
          keywordHash={keywordHash}
          updateFunc={() => setUpdate(update + 1)}
          rankLink={rankLink}
        />
      )}
    </>
  );
}

const mapStateToProps = (store) => {
  return {
    defaultAccount: store.pocket.defaultAccount,
  };
};

export default connect(mapStateToProps)(Market);

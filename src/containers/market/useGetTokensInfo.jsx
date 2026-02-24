import { ChainId, Fetcher, Route as RouteUniswap, Token, WETH } from '@uniswap/sdk';
import { useEffect, useState } from 'react';
import { DIVISOR_CYBER_G } from 'src/constants/config';
import { useQueryClient } from 'src/contexts/queryClient';
import { TOTAL_GOL_GENESIS_SUPPLY } from '../../utils/config';
import { convertResources } from '../../utils/utils';

const initValue = {
  supply: 0,
  price: 0,
  cap: 0,
  loading: true,
};

const initState = {
  gol: {
    ...initValue,
  },
  cyb: {
    ...initValue,
  },
  boot: {
    ...initValue,
  },
  hydrogen: {
    ...initValue,
  },
  milliampere: {
    ...initValue,
  },
  millivolt: {
    ...initValue,
  },
  tocyb: {
    ...initValue,
  },
};

function useGetCybernomics() {
  const queryClient = useQueryClient();
  const [cybernomics, setCybernomics] = useState(initState);
  const [poolsData, setPoolsData] = useState([]);
  const [totalSupplyData, setTotalSupplyData] = useState({});

  const getGOL = async () => {
    const GOL = new Token(ChainId.MAINNET, '0xF4ecdBa8ba4144Ff3a2d8792Cad9051431Aa4F64', 0);
    const pair = await Fetcher.fetchPairData(GOL, WETH[GOL.chainId]);

    const route = new RouteUniswap([pair], WETH[GOL.chainId]);
    const currentPriceGol = route.midPrice.invert().toSignificant(6);
    const gol = {
      supply: parseFloat(TOTAL_GOL_GENESIS_SUPPLY),
      price: currentPriceGol * DIVISOR_CYBER_G,
      cap: parseFloat(currentPriceGol * TOTAL_GOL_GENESIS_SUPPLY),
      loading: false,
    };
    setCybernomics((item) => ({
      ...item,
      gol,
    }));
  };

  useEffect(() => {
    getGOL();
  }, [getGOL]);

  // useEffect(() => {
  //   if (!marketData.loading) {
  //     console.log('marketData', marketData);
  //     const { currentPrice } = marketData;

  //     const supplyEUL = parseFloat(GENESIS_SUPPLY);
  //     const capETH = (supplyEUL / CYBER.DIVISOR_CYBER_G) * currentPrice;

  //     const cyb = {
  //       cap: capETH,
  //       price: currentPrice,
  //       supply: GENESIS_SUPPLY,
  //       loading: false,
  //     };

  //     setCybernomics((item) => ({
  //       ...item,
  //       cyb,
  //     }));
  //   }
  // }, [marketData]);

  useEffect(() => {
    const feachTotal = async () => {
      if (queryClient) {
        const totalCyb = {};

        const totalSupply = await queryClient.totalSupply();
        if (Object.keys(totalSupply).length > 0) {
          totalSupply.forEach((item) => {
            totalCyb[item.denom] = parseFloat(item.amount);
          });
        }

        setTotalSupplyData(totalCyb);

        const value = {
          supply: totalCyb.boot,
          price: 0,
          cap: 0,
          loading: false,
        };

        if (totalCyb.boot) {
          value.supply = totalCyb.boot;
          setCybernomics((item) => ({
            ...item,
            boot: {
              ...value,
            },
          }));
        }
        if (totalCyb.hydrogen) {
          value.supply = totalCyb.hydrogen;
          setCybernomics((item) => ({
            ...item,
            hydrogen: {
              ...value,
            },
          }));
        }
        if (totalCyb.milliampere) {
          value.supply = convertResources(totalCyb.milliampere);
          setCybernomics((item) => ({
            ...item,
            milliampere: {
              ...value,
            },
          }));
        }
        if (totalCyb.millivolt) {
          value.supply = convertResources(totalCyb.millivolt);
          setCybernomics((item) => ({
            ...item,
            millivolt: {
              ...value,
            },
          }));
        }
        if (totalCyb.tocyb) {
          value.supply = totalCyb.tocyb;
          setCybernomics((item) => ({
            ...item,
            tocyb: {
              ...value,
            },
          }));
        }
      }
    };
    feachTotal();
  }, [queryClient]);

  useEffect(() => {
    try {
      const getPools = async () => {
        if (queryClient) {
          const responsePools = await queryClient.pools();
          setPoolsData(responsePools.pools);
        }
      };
      getPools();
    } catch (_error) {
      setPoolsData([]);
    }
  }, [queryClient]);

  return {
    ...cybernomics,
    poolsData,
    totalSupplyData,
  };
}

export default useGetCybernomics;

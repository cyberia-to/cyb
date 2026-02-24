import { Pool } from '@cybercongress/cyber-js/build/codec/tendermint/liquidity/v1beta1/liquidity';
import { Pane } from '@cybercongress/gravity';
import BigNumber from 'bignumber.js';
import React, { useCallback, useEffect, useRef, useState } from 'react';
import { useSelector } from 'react-redux';
import { createSearchParams, useParams, useSearchParams } from 'react-router-dom';
import { MainContainer } from 'src/components';
import { BASE_DENOM, DENOM_LIQUID } from 'src/constants/config';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { useQueryClient } from 'src/contexts/queryClient';
import { useAdviser } from 'src/features/adviser/context';
import useGetBalances from 'src/hooks/getBalances';
import useGetTotalSupply from 'src/hooks/useGetTotalSupply';
import usePoolListInterval from 'src/hooks/usePoolListInterval';
import { RootState } from 'src/redux/store';
import { Option } from 'src/types';
import useSetActiveAddress from '../../hooks/useSetActiveAddress';
import { useGetSwapPrice } from '../../pages/teleport/hooks';
import { sortReserveCoinDenoms } from '../../pages/teleport/swap/utils';
import { getDisplayAmountReverce, reduceBalances } from '../../utils/utils';
import ActionBar from './ActionBar';
import DepositCreatePool from './components/DepositCreatePool';
import TabList from './components/tabList';
import Withdraw from './components/withdraw';
import { MyPoolsT, TypeTab } from './type';
import { calculateCounterPairAmount, getMyTokenBalanceNumber, getPoolToken } from './utils';

const tokenADefaultValue = BASE_DENOM;
const tokenBDefaultValue = DENOM_LIQUID;

function Warp() {
  const queryClient = useQueryClient();
  const { tracesDenom } = useIbcDenom();
  const { defaultAccount } = useSelector((state: RootState) => state.pocket);
  const [searchParams, setSearchParams] = useSearchParams();
  const { tab = 'add-liquidity' } = useParams<{ tab: TypeTab }>();
  const { addressActive } = useSetActiveAddress(defaultAccount);
  const [_update, setUpdate] = useState(0);
  const { liquidBalances: accountBalances, refresh } = useGetBalances(addressActive?.bech32);
  const { totalSupplyProofList: totalSupply } = useGetTotalSupply();
  const poolsData = usePoolListInterval({ refetchInterval: 50000 });

  const [tokenA, setTokenA] = useState<string>(tokenADefaultValue);
  const [tokenB, setTokenB] = useState<string>(tokenBDefaultValue);
  const [tokenAAmount, setTokenAAmount] = useState<string | number>('');
  const [tokenBAmount, setTokenBAmount] = useState<string | number>('');
  const [tokenAPoolAmount, setTokenAPoolAmount] = useState<number>(0);
  const [tokenBPoolAmount, setTokenBPoolAmount] = useState<number>(0);
  const [tokenACoinDecimals, setTokenACoinDecimals] = useState<number>(0);
  const [tokenBCoinDecimals, setTokenBCoinDecimals] = useState<number>(0);
  const [selectedPool, setSelectedPool] = useState<Pool | undefined>(undefined);
  const [isExceeded, setIsExceeded] = useState<boolean>(false);
  const [isEmptyPool, setIsEmptyPool] = useState<boolean>(false);
  const [amountPoolCoin, setAmountPoolCoin] = useState<string | number>('');
  const [myPools, setMyPools] = useState<Option<{ [key: string]: MyPoolsT }>>(undefined);
  const [selectMyPool, setSelectMyPool] = useState('');

  const swapPrice = useGetSwapPrice(tokenA, tokenB, tokenAPoolAmount, tokenBPoolAmount);
  const firstEffectOccured = useRef(false);

  const { setAdviser } = useAdviser();

  useEffect(() => {
    let text;

    switch (tab) {
      case 'add-liquidity':
        text = 'play with pools earn more values';
        break;
      case 'create-pool':
        text = (
          <>
            the unlimited number of variations. combine your favorite tokens <br /> cultivate your
            values. place of cyber alchemists
          </>
        );

        break;
      case 'sub-liquidity':
        text = 'manage your liquidity';
        break;

      default:
        break;
    }

    setAdviser(text);
  }, [setAdviser, tab]);

  useEffect(() => {
    const [{ coinDecimals }] = tracesDenom(tokenA);
    setTokenACoinDecimals(coinDecimals);
  }, [tracesDenom, tokenA]);

  useEffect(() => {
    const [{ coinDecimals }] = tracesDenom(tokenB);
    setTokenBCoinDecimals(coinDecimals);
  }, [tracesDenom, tokenB]);

  useEffect(() => {
    if (firstEffectOccured.current) {
      // eslint-disable-next-line react-hooks/exhaustive-deps
      const query = {
        from: tokenA,
        to: tokenB,
      };

      setSearchParams(createSearchParams(query));
    } else {
      firstEffectOccured.current = true;
      const param = Object.fromEntries(searchParams.entries());
      if (Object.keys(param).length > 0) {
        const { from, to } = param;
        setTokenA(from);
        setTokenB(to);
      }
    }

    if (tab === 'sub-liquidity') {
      setSearchParams(createSearchParams({}));
    }
  }, [tokenA, tokenB, setSearchParams, searchParams, tab]);

  useEffect(() => {
    // find pool for current pair
    setSelectedPool(undefined);

    if (!poolsData || !poolsData.length) {
      return;
    }

    if (tokenA.length > 0 && tokenB.length > 0) {
      const findPool = poolsData.find(
        (item) =>
          sortReserveCoinDenoms(item.reserveCoinDenoms[0], item.reserveCoinDenoms[1]).join() ===
          sortReserveCoinDenoms(tokenA, tokenB).join()
      );
      setSelectedPool(findPool);
    }
  }, [poolsData, tokenA, tokenB]);

  useEffect(() => {
    (async () => {
      setTokenAPoolAmount(0);
      setTokenBPoolAmount(0);
      setIsEmptyPool(false);

      if (!queryClient || !selectedPool) {
        return;
      }

      const getAllBalancesPromise = await queryClient.getAllBalances(
        selectedPool.reserveAccountAddress
      );

      setIsEmptyPool(!getAllBalancesPromise.length);

      const dataReduceBalances = reduceBalances(getAllBalancesPromise);

      setTokenAPoolAmount(dataReduceBalances[tokenA] || 0);
      setTokenBPoolAmount(dataReduceBalances[tokenB] || 0);
    })();
  }, [queryClient, tokenA, tokenB, selectedPool]);

  useEffect(() => {
    if (accountBalances !== null && poolsData && poolsData !== null) {
      const poolTokenData = getPoolToken(poolsData, accountBalances);
      let poolTokenDataIndexer = {};

      poolTokenDataIndexer = poolTokenData.reduce(
        (obj, item) => ({
          ...obj,
          [item.poolCoinDenom]: item,
        }),
        {}
      );
      setMyPools(poolTokenDataIndexer);
    }
  }, [accountBalances, poolsData]);

  useEffect(() => {
    let exceeded = true;
    const myATokenBalance = getMyTokenBalanceNumber(tokenA, accountBalances);
    const myATokenBalanceB = getMyTokenBalanceNumber(tokenB, accountBalances);

    if (accountBalances !== null) {
      const validTokensAB =
        Object.hasOwn(accountBalances, tokenA) &&
        Object.hasOwn(accountBalances, tokenB) &&
        accountBalances[tokenA] > 0 &&
        accountBalances[tokenB] > 0;

      const validTokenAmountAB =
        parseFloat(getDisplayAmountReverce(tokenAAmount, tokenACoinDecimals)) <= myATokenBalance &&
        Number(tokenAAmount) > 0 &&
        parseFloat(getDisplayAmountReverce(tokenBAmount, tokenBCoinDecimals)) <= myATokenBalanceB &&
        Number(tokenBAmount) > 0;

      const resultValidSelectTokens = validTokensAB && validTokenAmountAB;

      if (tab === 'add-liquidity' && resultValidSelectTokens && swapPrice !== 0) {
        exceeded = false;
      }

      // valid add-liquidity in empty pool
      if (tab === 'add-liquidity' && isEmptyPool && resultValidSelectTokens && swapPrice === 0) {
        exceeded = false;
      }

      if (tab === 'create-pool' && resultValidSelectTokens) {
        exceeded = false;
      }
    }
    setIsExceeded(exceeded);
  }, [
    accountBalances,
    tokenA,
    tokenB,
    tab,
    tokenAAmount,
    tokenBAmount,
    swapPrice,
    tokenACoinDecimals,
    tokenBCoinDecimals,
    isEmptyPool,
  ]);

  const amountChangeHandler = useCallback(
    (inputAmount: string, e: React.ChangeEvent) => {
      let counterPairValue = new BigNumber(0);
      const isReverse = e.target.id !== 'tokenAAmount';

      if (Number(inputAmount) > 0 && tokenAPoolAmount && tokenBPoolAmount) {
        const state = {
          tokenAPoolAmount,
          tokenBPoolAmount,
          tokenA,
          tokenB,
          tokenACoinDecimals,
          tokenBCoinDecimals,
          isReverse,
        };

        const { counterPairAmount } = calculateCounterPairAmount(inputAmount, state);

        counterPairValue = counterPairAmount;
      }

      if (isReverse) {
        setTokenBAmount(new BigNumber(inputAmount).toNumber());
        setTokenAAmount(counterPairValue.toString(10));
      } else {
        setTokenAAmount(new BigNumber(inputAmount).toNumber());
        setTokenBAmount(counterPairValue.toString(10));
      }
    },
    [tokenAPoolAmount, tokenBPoolAmount, tokenB, tokenA, tokenACoinDecimals, tokenBCoinDecimals]
  );

  const amountChangeHandlerCreatePool = useCallback((values: string, e: React.ChangeEvent) => {
    const inputAmount = values;

    const isReverse = e.target.id !== 'tokenAAmount';

    if (isReverse) {
      setTokenBAmount(new BigNumber(inputAmount).toNumber());
    } else {
      setTokenAAmount(new BigNumber(inputAmount).toNumber());
    }
  }, []);

  const onChangeInputWithdraw = (values: string) => {
    const inputAmount = values;

    const myATokenBalance = getMyTokenBalanceNumber(selectMyPool, accountBalances);
    let exceeded = true;
    if (parseFloat(inputAmount) <= myATokenBalance && parseFloat(inputAmount) > 0) {
      exceeded = false;
    }
    setIsExceeded(exceeded);
    setAmountPoolCoin(new BigNumber(inputAmount).toNumber());
  };

  function tokenChange() {
    const A = tokenB;
    const B = tokenA;
    const AP = tokenBPoolAmount;
    const BP = tokenAPoolAmount;

    setTokenA(A);
    setTokenB(B);
    setTokenAAmount('');
    setTokenBAmount('');
    setTokenAPoolAmount(AP);
    setTokenBPoolAmount(BP);
  }

  const updateFunc = useCallback(() => {
    setUpdate((item) => item + 1);
    refresh();
  }, [refresh]);

  const stateProps = {
    accountBalances,
    tokenAAmount,
    tokenBAmount,
    tokenA,
    tokenB,
    setTokenA,
    setTokenB,
    totalSupply,
    tokenChange,
  };

  const stateWithdraw = {
    accountBalances,
    myPools,
    selectMyPool,
    setSelectMyPool,
    amountPoolCoin,
    onChangeInputWithdraw,
  };

  const stateActionBar = {
    tokenAAmount,
    tokenBAmount,
    tokenA,
    tokenB,
    selectedPool,
    updateFunc,
    isExceeded,
    tab,
    amountPoolCoin,
    myPools,
    selectMyPool,
  };

  return (
    <>
      <MainContainer width="62%">
        <TabList selected={tab} />
        <Pane
          width="375px"
          display="flex"
          alignItems="center"
          flexDirection="column"
          marginX="auto"
        >
          <Pane width="100%" display="flex" alignItems="center" flexDirection="column">
            {tab === 'add-liquidity' && (
              <DepositCreatePool
                stateProps={stateProps}
                amountChangeHandler={
                  isEmptyPool ? amountChangeHandlerCreatePool : amountChangeHandler
                }
              />
            )}
            {tab === 'create-pool' && (
              <DepositCreatePool
                stateProps={stateProps}
                amountChangeHandler={amountChangeHandlerCreatePool}
              />
            )}
            {tab === 'sub-liquidity' && <Withdraw stateProps={stateWithdraw} />}
          </Pane>
        </Pane>

        {/* <TraceTxTable /> */}
      </MainContainer>
      <ActionBar stateActionBar={stateActionBar} />
    </>
  );
}

export default Warp;

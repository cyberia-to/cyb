import { Pool } from '@cybercongress/cyber-js/build/codec/tendermint/liquidity/v1beta1/liquidity';
import BigNumber from 'bignumber.js';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createSearchParams, useSearchParams } from 'react-router-dom';
import { MainContainer, Slider } from 'src/components';
import { BASE_DENOM, DENOM_LIQUID } from 'src/constants/config';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { useQueryClient } from 'src/contexts/queryClient';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import usePoolListInterval from 'src/hooks/usePoolListInterval';
import useSetActiveAddress from 'src/hooks/useSetActiveAddress';
import { useAppSelector } from 'src/redux/hooks';
import { RootState } from 'src/redux/store';
import { getDisplayAmount, getDisplayAmountReverce, reduceBalances } from 'src/utils/utils';
import { TeleportContainer } from '../components/containers/Containers';
import { useGetParams, useGetSwapPrice } from '../hooks';
import useGetSendTxsByAddressByType from '../hooks/useGetSendTxsByAddress';
import { useTeleport } from '../Teleport.context';
import ActionBar from './actionBar.swap';
import DataSwapTxs from './components/dataSwapTxs/DataSwapTxs';
import Slippage from './components/slippage/Slippage';
import TokenSetterSwap, { TokenSetterId } from './components/TokenSetterSwap';
import { calculatePairAmount, sortReserveCoinDenoms } from './utils';

const tokenADefaultValue = BASE_DENOM;
const tokenBDefaultValue = DENOM_LIQUID;

function Swap() {
  const { tracesDenom } = useIbcDenom();
  const { totalSupplyProofList: totalSupply, accountBalances, refreshBalances } = useTeleport();
  const queryClient = useQueryClient();
  const [update, setUpdate] = useState(0);
  const { defaultAccount } = useAppSelector((state: RootState) => state.pocket);
  const { addressActive } = useSetActiveAddress(defaultAccount);
  const dataSwapTxs = useGetSendTxsByAddressByType(
    addressActive,
    'tendermint.liquidity.v1beta1.MsgSwapWithinBatch'
  );
  const poolsData = usePoolListInterval({ refetchInterval: 5 * 60 * 1000 });
  const params = useGetParams();
  const [searchParams, setSearchParams] = useSearchParams();
  const [tokenA, setTokenA] = useState<string>(tokenADefaultValue);
  const [tokenB, setTokenB] = useState<string>(tokenBDefaultValue);
  const [tokenAAmount, setTokenAAmount] = useState<string>('');
  const [tokenBAmount, setTokenBAmount] = useState<string>('');
  const [tokenAPoolAmount, setTokenAPoolAmount] = useState<number>(0);
  const [tokenBPoolAmount, setTokenBPoolAmount] = useState<number>(0);
  const [selectedPool, setSelectedPool] = useState<Pool | undefined>(undefined);
  const [swapPrice, setSwapPrice] = useState<number>(0);
  const [isExceeded, setIsExceeded] = useState<boolean>(false);
  const poolPrice = useGetSwapPrice(tokenA, tokenB, tokenAPoolAmount, tokenBPoolAmount);
  const firstEffectOccured = useRef(false);
  const skipRecalc = useRef(false);
  const tokenAAmountRef = useRef('');
  const [tokenABalance, setTokenABalance] = useState(0);
  const [tokenBBalance, setTokenBBalance] = useState(0);

  const [tokenACoinDecimals, setTokenACoinDecimals] = useState<number>(0);
  const [tokenBCoinDecimals, setTokenBCoinDecimals] = useState<number>(0);

  useEffect(() => {
    const [{ coinDecimals }] = tracesDenom(tokenA);
    setTokenACoinDecimals(coinDecimals);
  }, [tracesDenom, tokenA]);

  useEffect(() => {
    const [{ coinDecimals }] = tracesDenom(tokenB);
    setTokenBCoinDecimals(coinDecimals);
  }, [tracesDenom, tokenB]);

  useEffect(() => {
    const balance = accountBalances ? accountBalances[tokenA] || 0 : 0;
    setTokenABalance(balance);
  }, [accountBalances, tokenA]);

  useEffect(() => {
    const balance = accountBalances ? accountBalances[tokenB] || 0 : 0;
    setTokenBBalance(balance);
  }, [accountBalances, tokenB]);

  useEffect(() => {
    // find pool for current pair
    setSelectedPool(undefined);
    if (poolsData && poolsData.length > 0) {
      if (tokenA.length > 0 && tokenB.length > 0) {
        poolsData.forEach((item) => {
          if (
            sortReserveCoinDenoms(item.reserveCoinDenoms[0], item.reserveCoinDenoms[1]).join() ===
            sortReserveCoinDenoms(tokenA, tokenB).join()
          ) {
            setSelectedPool(item);
          }
        });
      }
    }
  }, [poolsData, tokenA, tokenB]);

  useEffect(() => {
    (async () => {
      setTokenAPoolAmount(0);
      setTokenBPoolAmount(0);

      const isInitialized = queryClient && selectedPool;

      if (!isInitialized) {
        return;
      }

      const getAllBalancesPromise = await queryClient.getAllBalances(
        selectedPool.reserveAccountAddress
      );
      const dataReduceBalances = reduceBalances(getAllBalancesPromise);

      setTokenAPoolAmount(dataReduceBalances[tokenA] || 0);
      setTokenBPoolAmount(dataReduceBalances[tokenB] || 0);
    })();
  }, [queryClient, tokenA, tokenB, selectedPool]);

  const amountChangeHandler = useCallback(
    (values: string | number, id: TokenSetterId) => {
      const inputAmount = values;
      let counterPairAmount = new BigNumber(0);

      const isReverse = id !== TokenSetterId.tokenAAmount;

      if (tokenAPoolAmount && tokenAPoolAmount && Number(inputAmount) > 0) {
        const state = {
          tokenB,
          tokenA,
          tokenBPoolAmount,
          tokenAPoolAmount,
          coinDecimalsA: tokenACoinDecimals,
          coinDecimalsB: tokenBCoinDecimals,
          isReverse,
        };

        const { counterPairAmount: counterPairAmountValue, price } = calculatePairAmount(
          inputAmount,
          state
        );

        counterPairAmount = counterPairAmountValue;
        setSwapPrice(price.toNumber());
      } else {
        setSwapPrice(0);
      }

      const counterPairStr = counterPairAmount.toString(10);
      if (isReverse) {
        setTokenBAmount(String(inputAmount));
        setTokenAAmount(counterPairStr);
      } else {
        setTokenAAmount(String(inputAmount));
        setTokenBAmount(counterPairStr);
      }
    },
    [tokenAPoolAmount, tokenB, tokenA, tokenBPoolAmount, tokenACoinDecimals, tokenBCoinDecimals]
  );

  // Recalculate only the output (tokenB + swapPrice) without touching tokenA.
  // This prevents the feedback loop where setting tokenA triggers InputNumber
  // re-render → onValueChange → amountChangeHandler → setTokenA → loop.
  const recalcOutput = useCallback(
    (inputAmount: string) => {
      if (tokenAPoolAmount && tokenBPoolAmount && Number(inputAmount) > 0) {
        const state = {
          tokenB,
          tokenA,
          tokenBPoolAmount,
          tokenAPoolAmount,
          coinDecimalsA: tokenACoinDecimals,
          coinDecimalsB: tokenBCoinDecimals,
          isReverse: false,
        };

        const { counterPairAmount, price } = calculatePairAmount(inputAmount, state);
        setSwapPrice(price.toNumber());
        setTokenBAmount(counterPairAmount.toString(10));
      } else {
        setSwapPrice(0);
        setTokenBAmount('0');
      }
    },
    [tokenAPoolAmount, tokenB, tokenA, tokenBPoolAmount, tokenACoinDecimals, tokenBCoinDecimals]
  );

  const recalcOutputRef = useRef<typeof recalcOutput>(null!);
  useEffect(() => {
    recalcOutputRef.current = recalcOutput;
  }, [recalcOutput]);

  // Keep tokenAAmountRef in sync
  useEffect(() => {
    tokenAAmountRef.current = tokenAAmount;
  }, [tokenAAmount]);

  useEffect(() => {
    // Recalculate output when pool data changes or update is triggered.
    // tokenAAmount is read from ref to avoid being a dependency —
    // user input already calls amountChangeHandler directly.
    if (skipRecalc.current) {
      skipRecalc.current = false;
      return;
    }
    const amount = tokenAAmountRef.current;
    if (Number(amount) > 0) {
      recalcOutputRef.current(amount);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [update, tokenAPoolAmount, tokenBPoolAmount]);

  const validInputAmountTokenA = useMemo(() => {
    const isValid = Number(tokenAAmount) > 0 && !!tokenABalance;

    if (!isValid) {
      return false;
    }

    const amountToken = parseFloat(getDisplayAmountReverce(tokenAAmount, tokenACoinDecimals));

    return amountToken > tokenABalance;
  }, [tokenAAmount, tokenABalance, tokenACoinDecimals]);

  const useGetSlippage = useMemo(() => {
    if (poolPrice && swapPrice) {
      // poolPrice / price - 1

      const slippage = new BigNumber(poolPrice)
        .dividedBy(swapPrice)
        .minus(1)
        .multipliedBy(100)
        .dp(2, BigNumber.ROUND_FLOOR);

      if (slippage.comparedTo(0) < 0) {
        return slippage.multipliedBy(-1).toNumber();
      }
      return slippage.toNumber();
    }
    return 0;
  }, [poolPrice, swapPrice]);

  const CHAIN_MIN_OFFER_AMOUNT = 100; // chain rejects offers below 100 base units

  const belowMinAmount = useMemo(() => {
    if (!tokenAAmount || Number(tokenAAmount) <= 0) {
      return false;
    }
    const rawAmount = new BigNumber(tokenAAmount).shiftedBy(tokenACoinDecimals);
    return rawAmount.isLessThan(CHAIN_MIN_OFFER_AMOUNT);
  }, [tokenAAmount, tokenACoinDecimals]);

  const exceedsMaxOrderRatio = useMemo(() => {
    if (!tokenAPoolAmount || !tokenAAmount || !tokenACoinDecimals) {
      return false;
    }
    const rawAmount = new BigNumber(tokenAAmount).shiftedBy(tokenACoinDecimals);
    const maxOrderable = new BigNumber(tokenAPoolAmount).multipliedBy(0.1);
    return rawAmount.isGreaterThan(maxOrderable);
  }, [tokenAAmount, tokenAPoolAmount, tokenACoinDecimals]);

  useEffect(() => {
    // validation swap
    let exceeded = true;

    const validTokenAmountA = !validInputAmountTokenA && Number(tokenAAmount) > 0;

    // check pool, check slippage 3%, check max order ratio 10%, check min amount
    if (poolPrice !== 0 && validTokenAmountA && useGetSlippage < 3 && !exceedsMaxOrderRatio && !belowMinAmount) {
      exceeded = false;
    }

    setIsExceeded(exceeded);
  }, [poolPrice, tokenAAmount, validInputAmountTokenA, useGetSlippage, exceedsMaxOrderRatio, belowMinAmount]);

  const pairPrice = useMemo(() => {
    const isValid = poolPrice && tokenA && tokenB;
    const pair = { priceA: 0, priceB: 0, tokenA, tokenB };

    if (!isValid) {
      return pair;
    }

    let revPrice = new BigNumber(0);
    let position = 0;

    if ([tokenA, tokenB].sort()[0] === tokenA) {
      revPrice = new BigNumber(1).dividedBy(poolPrice);
      position = tokenBCoinDecimals;
    } else {
      position = tokenACoinDecimals;
      const amountTokenA = getDisplayAmountReverce(1, position);
      revPrice = new BigNumber(amountTokenA).multipliedBy(poolPrice);
    }

    if (!position || revPrice) {
      revPrice.dp(position, BigNumber.ROUND_FLOOR);
    }

    pair.priceA = 1;
    pair.priceB = revPrice.toNumber();

    return pair;
  }, [poolPrice, tokenA, tokenACoinDecimals, tokenB, tokenBCoinDecimals]);

  function tokenChange() {
    const A = tokenB;
    const B = tokenA;

    setTokenA(A);
    setTokenB(B);
    setTokenAAmount('');
    setTokenBAmount('');
  }

  const updateFunc = useCallback(() => {
    skipRecalc.current = true;
    setTokenAAmount('');
    setTokenBAmount('');
    setSwapPrice(0);
    setUpdate((item) => item + 1);
    dataSwapTxs.refetch();
    refreshBalances();
  }, [dataSwapTxs, refreshBalances]);

  const setPercentageBalanceHook = useCallback(
    (value: number) => {
      const amount = new BigNumber(tokenABalance)
        .multipliedBy(value)
        .dividedBy(100)
        .dp(tokenACoinDecimals, BigNumber.ROUND_FLOOR)
        .toNumber();
      const amountDecimals = getDisplayAmount(amount, tokenACoinDecimals);
      amountChangeHandler(amountDecimals, TokenSetterId.tokenAAmount);
      setTokenAAmount(amountDecimals);
    },
    [tokenABalance, tokenACoinDecimals, amountChangeHandler]
  );

  useEffect(() => {
    if (firstEffectOccured.current) {
      setSearchParams(createSearchParams({ from: tokenA, to: tokenB }), { replace: true });
    } else {
      firstEffectOccured.current = true;
      const param = Object.fromEntries(searchParams.entries());
      if (Object.keys(param).length > 0) {
        const { from, to } = param;
        if (from) setTokenA(from);
        if (to) setTokenB(to);
        // clean URL: remove stale params like amount
        setSearchParams(createSearchParams({ from: from || tokenA, to: to || tokenB }), { replace: true });
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tokenA, tokenB, setSearchParams]);

  const getPercentsOfToken = useCallback(() => {
    return tokenABalance > 0
      ? new BigNumber(getDisplayAmountReverce(tokenAAmount, tokenACoinDecimals))
          .dividedBy(tokenABalance)
          .multipliedBy(100)
          .toNumber()
      : 0;
  }, [tokenAAmount, tokenACoinDecimals, tokenABalance]);

  const stateActionBar = {
    tokenAAmount,
    tokenA,
    tokenB,
    params,
    selectedPool,
    updateFunc,
    isExceeded,
    swapPrice,
    poolPrice,
  };

  const adviserText = useMemo(() => {
    if (belowMinAmount) {
      return 'amount too small: minimum 100 base units required by chain';
    }
    if (exceedsMaxOrderRatio) {
      return 'amount exceeds 10% of pool reserves';
    }
    return 'swap tokens';
  }, [belowMinAmount, exceedsMaxOrderRatio]);

  useAdviserTexts({
    defaultText: adviserText,
  });

  return (
    <>
      <MainContainer>
        <TeleportContainer>
          <TokenSetterSwap
            id={TokenSetterId.tokenAAmount}
            listTokens={totalSupply}
            amountToken={getDisplayAmount(tokenABalance, tokenACoinDecimals)}
            tokenAmountValue={tokenAAmount}
            valueSelect={tokenA}
            selected={tokenB}
            onChangeSelect={setTokenA}
            amountChangeHandler={amountChangeHandler}
            validInputAmount={validInputAmountTokenA}
            warningAmount={exceedsMaxOrderRatio}
            warningAmountText="exceeds 10% of pool reserves"
            autoFocus
          />

          <Slider
            valuePercents={getPercentsOfToken()}
            onChange={setPercentageBalanceHook}
            onSwapClick={() => tokenChange()}
            tokenPair={pairPrice}
            text={
              <>
                <Slippage value={useGetSlippage} />
                {Number(tokenAAmount) > 0 && (
                  <span style={{ color: 'var(--grayscale-dark)', fontSize: '0.75rem', marginLeft: 8 }}>
                    fee (0.3%): <span style={{ color: 'var(--grayscale-primary)' }}>{new BigNumber(tokenAAmount).multipliedBy(0.003).dp(tokenACoinDecimals > 0 ? tokenACoinDecimals : 2).toString()}</span>
                  </span>
                )}
              </>
            }
          />

          <TokenSetterSwap
            id={TokenSetterId.tokenBAmount}
            listTokens={totalSupply}
            amountToken={getDisplayAmount(tokenBBalance, tokenBCoinDecimals)}
            tokenAmountValue={tokenBAmount}
            valueSelect={tokenB}
            selected={tokenA}
            onChangeSelect={setTokenB}
            amountChangeHandler={amountChangeHandler}
            validAmountMessage={!selectedPool}
            validAmountMessageText="no pool"
          />
        </TeleportContainer>
        <TeleportContainer>
          <DataSwapTxs dataTxs={dataSwapTxs} />
        </TeleportContainer>
      </MainContainer>
      <ActionBar stateActionBar={stateActionBar} />
    </>
  );
}

export default Swap;

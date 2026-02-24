import BigNumber from 'bignumber.js';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createSearchParams, useSearchParams } from 'react-router-dom';
import { AvailableAmount, DenomArr, MainContainer, Slider } from 'src/components';
import Select, { OptionSelect } from 'src/components/Select';
import { CHAIN_ID } from 'src/constants/config';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import { useChannels } from 'src/hooks/useHub';
import { Networks } from 'src/types/networks';
import { getDisplayAmount, getDisplayAmountReverce } from 'src/utils/utils';
import HistoryContextProvider from '../../../features/ibc-history/historyContext';
import { Col, GridContainer, TeleportContainer } from '../components/containers/Containers';
import InputNumberDecimalScale from '../components/Inputs/InputNumberDecimalScale/InputNumberDecimalScale';
import { useSetupIbcClient } from '../hooks';
import { useTeleport } from '../Teleport.context';
import { TypeTxsT } from '../type';
import ActionBar from './actionBar.bridge';
import DataIbcHistory from './components/dataIbcHistory/DataIbcHistory';

type Query = {
  networkFrom: string;
  networkTo: string;
  token: string;
  amount?: string;
};

export const ibcDenomAtom = 'ibc/15E9C5CF5969080539DB395FA7D9C0868265217EFC528433671AAF9B1912D159';

const isCyberChain = (chainId: string) => chainId === CHAIN_ID;

function Bridge() {
  const { tracesDenom } = useIbcDenom();
  const { channels } = useChannels();
  const { totalSupplyProofList, accountBalances, refreshBalances } = useTeleport();
  const [searchParams, setSearchParams] = useSearchParams();

  const [tokenSelect, setTokenSelect] = useState<string>(ibcDenomAtom);
  const [tokenAmount, setTokenAmount] = useState<string>('');
  const [networkA, setNetworkA] = useState<string>(Networks.COSMOS);
  const [networkB, setNetworkB] = useState<string>(Networks.BOSTROM);
  const [typeTxs, setTypeTxs] = useState<TypeTxsT>('deposit');
  const [sourceChannel, setSourceChannel] = useState<string | null>(null);
  const [isExceeded, setIsExceeded] = useState<boolean>(false);

  const [tokenA, setTokenA] = useState<string>('');
  const [tokenB, setTokenB] = useState<string>('');

  const [tokenABalance, setTokenABalance] = useState<number>(0);
  const [tokenBBalance, setTokenBBalance] = useState<number>(0);

  const [tokenACoinDecimals, setTokenACoinDecimals] = useState<number>(0);

  const { ibcClient, balanceIbc, denomIbc } = useSetupIbcClient(
    tokenSelect,
    typeTxs === 'deposit' ? networkA : networkB
  );

  const firstEffectOccured = useRef(false);

  useEffect(() => {
    if (firstEffectOccured.current) {
      // eslint-disable-next-line react-hooks/exhaustive-deps
      const query: Query = {
        networkFrom: networkA,
        networkTo: networkB,
        token: tokenSelect,
      };

      if (Number(tokenAmount) > 0) {
        query.amount = tokenAmount;
      }

      setSearchParams(createSearchParams(query), { replace: true });
    } else {
      firstEffectOccured.current = true;
      const param = Object.fromEntries(searchParams.entries());
      if (Object.keys(param).length > 0) {
        const { networkFrom, networkTo, token, amount } = param;
        setNetworkA(networkFrom);
        setNetworkB(networkTo);
        setTokenSelect(token);

        if (Number(amount)) {
          setTokenAmount(amount);
        }
      }
    }
  }, [networkA, networkB, tokenSelect, setSearchParams, searchParams, tokenAmount]);

  useEffect(() => {
    const isInitialized = channels && networkA && networkB;
    if (!isInitialized) {
      return;
    }

    if (!isCyberChain(networkA) && isCyberChain(networkB)) {
      setTypeTxs('deposit');
      const { destination_channel_id: destChannelId } = channels[networkA];
      setSourceChannel(destChannelId);
    }

    if (isCyberChain(networkA) && !isCyberChain(networkB)) {
      setTypeTxs('withdraw');

      const { source_channel_id: sourceChannelId } = channels[networkB];
      setSourceChannel(sourceChannelId);
    }
  }, [networkB, networkA, channels]);

  const networkOptions = useCallback(
    () =>
      channels
        ? [CHAIN_ID, ...Object.keys(channels)].map((key) => ({
            value: key,
            text: <DenomArr type="network" denomValue={key} onlyText tooltipStatusText={false} />,
            img: <DenomArr type="network" denomValue={key} onlyImg tooltipStatusImg={false} />,
          }))
        : [],
    [channels]
  );

  const tokenOptions = useMemo(
    () =>
      totalSupplyProofList
        ? Object.keys(totalSupplyProofList).map((key) => ({
            value: key,
            text: <DenomArr denomValue={key} onlyText tooltipStatusText={tokenSelect !== key} />,
            img: <DenomArr denomValue={key} onlyImg tooltipStatusImg={false} />,
          }))
        : [],

    [totalSupplyProofList, tokenSelect]
  );

  const validInputAmountToken = useMemo(() => {
    const isValid = Number(tokenAmount) > 0 && !!tokenAmount;

    if (!isValid) {
      return false;
    }

    const amountToken = parseFloat(getDisplayAmountReverce(tokenAmount, tokenACoinDecimals));

    return amountToken > tokenABalance;
  }, [tokenABalance, tokenAmount, tokenACoinDecimals]);

  useEffect(() => {
    const validNetwork = networkA && networkB && networkA !== networkB;

    // check set up ibc cliet for deposit
    const validIbcClient = typeTxs === 'deposit' ? ibcClient !== null : true;

    const validTokenAmount = !validInputAmountToken && Number(tokenAmount) > 0;

    setIsExceeded(!(validNetwork && validIbcClient && validTokenAmount));
  }, [networkA, networkB, typeTxs, ibcClient, tokenAmount, validInputAmountToken]);

  const setPercentageBalanceHook = useCallback(
    (value: number) => {
      const amount = new BigNumber(tokenABalance)
        .multipliedBy(value)
        .dividedBy(100)
        .dp(tokenACoinDecimals, BigNumber.ROUND_FLOOR)
        .toNumber();
      setTokenAmount(getDisplayAmount(amount, tokenACoinDecimals));
    },
    [tokenABalance, tokenACoinDecimals]
  );

  const getPercentsOfToken = useCallback(() => {
    return tokenABalance > 0
      ? new BigNumber(getDisplayAmountReverce(tokenAmount, tokenACoinDecimals))
          .dividedBy(tokenABalance)
          .multipliedBy(100)
          .toNumber()
      : 0;
  }, [tokenAmount, tokenACoinDecimals, tokenABalance]);

  const tokenChange = useCallback(() => {
    setNetworkA(networkB);
    setNetworkB(networkA);
    setTokenAmount(0);
  }, [networkB, networkA]);

  const getDenomToken = useCallback(
    (selectNetwork: string) => (!isCyberChain(selectNetwork) && denomIbc ? denomIbc : tokenSelect),
    [tokenSelect, denomIbc]
  );

  useEffect(() => {
    const [{ coinDecimals }] = tracesDenom(tokenSelect);
    setTokenACoinDecimals(coinDecimals);
  }, [tokenSelect, tracesDenom]);

  useEffect(() => {
    const token = getDenomToken(networkA);
    setTokenA(token);
  }, [networkA, getDenomToken]);

  useEffect(() => {
    const token = getDenomToken(networkB);
    setTokenB(token);
  }, [networkB, getDenomToken]);

  const getAccountBalancesToken = useCallback(
    (selectNetwork: string) => (!isCyberChain(selectNetwork) ? balanceIbc : accountBalances),
    [accountBalances, balanceIbc]
  );

  useEffect(() => {
    const balance = getAccountBalancesToken(networkA);
    const balanceToken = balance ? balance[tokenA] || 0 : 0;
    setTokenABalance(balanceToken);
  }, [getAccountBalancesToken, networkA, tokenA]);

  useEffect(() => {
    const balance = getAccountBalancesToken(networkB);
    const balanceToken = balance ? balance[tokenB] || 0 : 0;
    setTokenBBalance(balanceToken);
  }, [getAccountBalancesToken, networkB, tokenB]);

  useAdviserTexts({
    defaultText: 'bridge tokens',
  });

  const stateActionBar = {
    tokenAmount,
    tokenSelect,
    updateFunc: () => refreshBalances(),
    isExceeded,
    typeTxs,
    ibcClient,
    denomIbc,
    sourceChannel,
    networkB,
    coinDecimals: tokenACoinDecimals,
  };

  return (
    <HistoryContextProvider>
      <MainContainer width="62%">
        <TeleportContainer>
          <GridContainer>
            <Col>
              <InputNumberDecimalScale
                value={tokenAmount}
                onValueChange={(value) => setTokenAmount(value)}
                title="choose amount to send"
                validAmount={validInputAmountToken}
                tokenSelect={tokenSelect}
              />
              <AvailableAmount amountToken={getDisplayAmount(tokenABalance, tokenACoinDecimals)} />
            </Col>
            <Col>
              <Select
                valueSelect={tokenSelect}
                currentValue={
                  <OptionSelect
                    text="choose"
                    img={<DenomArr denomValue="choose" onlyImg />}
                    value=""
                    bgrImg
                  />
                }
                onChangeSelect={(item: string) => setTokenSelect(item)}
                width="100%"
                options={tokenOptions}
                title="choose token to send"
              />
              <Select
                valueSelect={networkA}
                currentValue={
                  <OptionSelect
                    text="choose"
                    img={<DenomArr denomValue="choose" onlyImg />}
                    value=""
                    bgrImg
                  />
                }
                onChangeSelect={(item: string) => setNetworkA(item)}
                width="100%"
                title="choose source network"
                options={networkOptions()}
              />
            </Col>
          </GridContainer>

          <Slider
            valuePercents={getPercentsOfToken()}
            onChange={setPercentageBalanceHook}
            onSwapClick={tokenChange}
            disabled={tokenABalance === 0}
          />

          <GridContainer>
            <AvailableAmount amountToken={getDisplayAmount(tokenBBalance, tokenACoinDecimals)} />
            <Select
              valueSelect={networkB}
              onChangeSelect={(item: string) => setNetworkB(item)}
              width="100%"
              options={networkOptions()}
              title="destination network"
            />
          </GridContainer>
        </TeleportContainer>
        <TeleportContainer>
          <DataIbcHistory />
        </TeleportContainer>
      </MainContainer>
      <ActionBar stateActionBar={stateActionBar} />
    </HistoryContextProvider>
  );
}

export default Bridge;

import BigNumber from 'bignumber.js';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createSearchParams, useSearchParams } from 'react-router-dom';
import { AvailableAmount, DenomArr, MainContainer, Select, Slider } from 'src/components';
import { OptionSelect } from 'src/components/Select';
import { BASE_DENOM, CHAIN_ID } from 'src/constants/config';
import { PATTERN_CYBER } from 'src/constants/patterns';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { useQueryClient } from 'src/contexts/queryClient';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import useAccountsPassports from 'src/features/passport/hooks/useAccountsPassports';
import useSetActiveAddress from 'src/hooks/useSetActiveAddress';
import { useAppSelector } from 'src/redux/hooks';
import { RootState } from 'src/redux/store';
import { Option } from 'src/types';
import { ObjKeyValue } from 'src/types/data';
import { getDisplayAmount, getDisplayAmountReverce, reduceBalances } from 'src/utils/utils';
import { Col, GridContainer, TeleportContainer } from '../components/containers/Containers';
import { AccountInput, InputMemo, InputNumberDecimalScale } from '../components/Inputs';
import useGetSendTxsByAddressByLcd from '../hooks/useGetSendTxsByAddressByLcd';
import { useTeleport } from '../Teleport.context';
import ActionBar from './actionBar.send';
import DataSendTxs from './components/dataSendTxs/DataSendTxs';

const tokenDefaultValue = BASE_DENOM;

function Send() {
  const queryClient = useQueryClient();
  const { tracesDenom } = useIbcDenom();
  const { defaultAccount } = useAppSelector((state: RootState) => state.pocket);
  useAccountsPassports();
  const { addressActive } = useSetActiveAddress(defaultAccount);
  const { totalSupplyProofList, accountBalances, refreshBalances } = useTeleport();
  const [_update, setUpdate] = useState(0);
  const [recipient, setRecipient] = useState<string | undefined>(undefined);
  const [searchParams, setSearchParams] = useSearchParams();
  // const dataSendTxs = useGetSendTxsByAddressByType(
  //   addressActive,
  //   'cosmos.bank.v1beta1.MsgSend'
  // );
  const dataSendTxs = useGetSendTxsByAddressByLcd(addressActive, recipient);
  const [tokenSelect, setTokenSelect] = useState<string>(tokenDefaultValue);
  const [tokenAmount, setTokenAmount] = useState<string>('');

  const [recipientBalances, setRecipientBalances] = useState<Option<ObjKeyValue>>(undefined);

  const [recipientTokenABalances, setRecipientTokenABalances] = useState(0);
  const [memoValue, setMemoValue] = useState<string>('');
  const [isExceeded, setIsExceeded] = useState<boolean>(false);
  const firstEffectOccured = useRef(false);

  const [tokenACoinDecimals, setTokenACoinDecimals] = useState<number>(0);
  const [tokenABalance, setTokenABalance] = useState<number>(0);

  useEffect(() => {
    if (firstEffectOccured.current) {
      // eslint-disable-next-line react-hooks/exhaustive-deps
      const query = {
        token: tokenSelect,
        recipient: '',
        amount: '',
      };

      if (recipient) {
        query.recipient = recipient;
      }

      if (Number(tokenAmount) > 0) {
        query.amount = tokenAmount;
      }

      setSearchParams(createSearchParams(query), { replace: true });
    } else {
      firstEffectOccured.current = true;
      const param = Object.fromEntries(searchParams.entries());
      if (Object.keys(param).length > 0) {
        const { token, recipient, amount } = param;
        setTokenSelect(token);
        if (recipient) {
          setRecipient(recipient);
        }

        if (amount && Number(amount) > 0) {
          setTokenAmount(amount);
        }
      }
    }
  }, [tokenSelect, recipient, setSearchParams, searchParams, tokenAmount]);

  useEffect(() => {
    const [{ coinDecimals }] = tracesDenom(tokenSelect);
    setTokenACoinDecimals(coinDecimals);
  }, [tokenSelect, tracesDenom]);

  // setTokenABalance
  useEffect(() => {
    if (accountBalances) {
      setTokenABalance(accountBalances[tokenSelect] || 0);
    }
  }, [tokenSelect, accountBalances]);

  const validInputAmountToken = useMemo(() => {
    if (Number(tokenAmount) > 0) {
      const amountToken = parseFloat(getDisplayAmountReverce(tokenAmount, tokenACoinDecimals));

      return amountToken > tokenABalance;
    }

    return false;
  }, [tokenAmount, tokenACoinDecimals, tokenABalance]);

  useEffect(() => {
    const validTokenAmount = !validInputAmountToken && Number(tokenAmount) > 0;
    const validRecipient = recipient?.match(PATTERN_CYBER);

    setIsExceeded(!(validRecipient && validTokenAmount));
  }, [recipient, validInputAmountToken, tokenAmount]);

  useEffect(() => {
    (async () => {
      setRecipientBalances(undefined);

      const isInit = queryClient && recipient && recipient.match(PATTERN_CYBER);

      if (!isInit) {
        return;
      }

      const getAllBalancesPromise = await queryClient.getAllBalances(recipient);
      const dataReduceBalances = reduceBalances(getAllBalancesPromise);

      setRecipientBalances(dataReduceBalances);
    })();
  }, [queryClient, recipient]);

  useEffect(() => {
    setRecipientTokenABalances(recipientBalances ? recipientBalances[tokenSelect] || 0 : 0);
  }, [recipientBalances, tokenSelect]);

  const reduceOptions = useMemo(
    () =>
      totalSupplyProofList
        ? Object.keys(totalSupplyProofList).map((key) => ({
            value: key,
            text: <DenomArr denomValue={key} onlyText tooltipStatusText={false} />,
            img: <DenomArr denomValue={key} onlyImg tooltipStatusImg={false} />,
          }))
        : [],

    [totalSupplyProofList]
  );

  const setPercentageBalanceHook = useCallback(
    (value: number) => {
      if (tokenABalance) {
        const amount = new BigNumber(tokenABalance)
          .multipliedBy(value)
          .dividedBy(100)
          .dp(tokenACoinDecimals, BigNumber.ROUND_FLOOR)
          .toNumber();

        setTokenAmount(getDisplayAmount(amount, tokenACoinDecimals));
      }
    },
    [tokenABalance, tokenACoinDecimals]
  );

  const getPercentsOfToken = useCallback(() => {
    const amountTokenA = getDisplayAmountReverce(tokenAmount, tokenACoinDecimals);

    return tokenABalance > 0
      ? new BigNumber(amountTokenA).dividedBy(tokenABalance).multipliedBy(100).toNumber()
      : 0;
  }, [tokenAmount, tokenABalance, tokenACoinDecimals]);

  const updateFunc = useCallback(() => {
    setUpdate((item) => item + 1);
    dataSendTxs.refetch();
    refreshBalances();
    setMemoValue('');
  }, [dataSendTxs, refreshBalances]);

  const amountTokenChange = useCallback(
    (tokenBalance: number, type: 'sender' | 'recipient') => {
      let amount = new BigNumber(getDisplayAmount(tokenBalance, tokenACoinDecimals));

      let changeAmount = new BigNumber(tokenAmount);

      if (type === 'sender') {
        changeAmount = changeAmount.multipliedBy(-1);
      }

      if (changeAmount.comparedTo(0)) {
        amount = new BigNumber(amount).plus(changeAmount);
      }

      return amount.comparedTo(0) >= 0 ? amount.toNumber() : 0;
    },
    [tokenAmount, tokenACoinDecimals]
  );

  useAdviserTexts({
    defaultText: 'send tokens',
  });

  const stateActionBar = {
    tokenAmount,
    tokenSelect,
    recipient,
    updateFunc,
    isExceeded,
    memoValue,
  };

  return (
    <>
      <MainContainer width="62%">
        <TeleportContainer>
          <Select
            valueSelect={CHAIN_ID}
            currentValue={CHAIN_ID}
            disabled
            options={[
              {
                value: CHAIN_ID,
                text: CHAIN_ID,
                img: (
                  <DenomArr denomValue={CHAIN_ID} onlyImg type="network" tooltipStatusImg={false} />
                ),
              },
            ]}
            width="100%"
            // disabled
            title="choose network"
          />
          <InputMemo value={memoValue} onChangeValue={setMemoValue} />
          <AccountInput recipient={recipient} setRecipient={setRecipient} />
          <GridContainer>
            <Col>
              <InputNumberDecimalScale
                value={tokenAmount}
                availableAmount={tokenABalance}
                onValueChange={(value) => setTokenAmount(value)}
                title="choose amount to send"
                validAmount={validInputAmountToken}
                tokenSelect={tokenSelect}
              />
              <AvailableAmount
                amountToken={amountTokenChange(tokenABalance, 'sender')}
                title={tokenAmount.length === 0 ? 'you have' : 'you will have'}
              />
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
                options={reduceOptions}
                title="choose token to send"
              />
              <AvailableAmount
                title={tokenAmount.length === 0 ? 'recipient have' : 'recipient will have'}
                amountToken={amountTokenChange(recipientTokenABalances, 'recipient')}
              />
            </Col>
          </GridContainer>

          <Slider valuePercents={getPercentsOfToken()} onChange={setPercentageBalanceHook} />
        </TeleportContainer>
        <TeleportContainer>
          <DataSendTxs dataSendTxs={dataSendTxs} accountUser={addressActive} />
        </TeleportContainer>
      </MainContainer>
      <ActionBar stateActionBar={stateActionBar} />
    </>
  );
}

export default Send;

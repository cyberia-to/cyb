import { QueryParamsResponse as QueryParamsResponseResources } from '@cybercongress/cyber-js/build/codec/cyber/resources/v1beta1/query';
import BigNumber from 'bignumber.js';
import { useEffect, useMemo, useState } from 'react';
import { DenomArr, Dots, FormatNumberTokens, MainContainer, Tabs, ValueImg } from 'src/components';
import Display from 'src/components/containerGradient/Display/Display';
import { DENOM_LIQUID } from 'src/constants/config';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { useQueryClient } from 'src/contexts/queryClient';
import { useAdviser } from 'src/features/adviser/context';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppSelector } from 'src/redux/hooks';
import { formatNumber, getDisplayAmount } from 'src/utils/utils';
import { TableSlots } from '../energy/ui';
import ActionBar from './actionBar';
import ERatio from './components/ERatio/ERatio';
import RcSlider from './components/Slider/Slider';
import InfoText from './InfoText/InfoText';
import LiquidBalances from './LiquidBalances/LiquidBalances';
import styles from './Mint.module.scss';
import Statistics from './Statistics/Statistics';
import { SelectedState } from './types';
import useGetSlots from './useGetSlots';
import { getAmountResource, getERatio, getMaxTimeMint, SLOTS_MAX } from './utils';

const returnColorDot = (marks) => {
  return {
    style: {
      color: '#fff',
      whiteSpace: 'nowrap',
    },
    label: marks,
  };
};

function Mint() {
  const queryClient = useQueryClient();
  const { tracesDenom } = useIbcDenom();
  const [_updateAddress, setUpdateAddress] = useState(0);
  const [selected, setSelected] = useState<SelectedState>(SelectedState.milliampere);
  const [value, setValue] = useState(0);
  const [valueDays, setValueDays] = useState(1);
  const [height, setHeight] = useState(0);
  const [resourcesParams, setResourcesParams] =
    useState<QueryParamsResponseResources['params']>(undefined);
  const [balanceHydrogen, SetBalanceHydrogen] = useState(0);

  const addressActive = useAppSelector(selectCurrentAddress);
  const { slotsData, vested, loadingAuthAccounts, originalVesting, update } =
    useGetSlots(addressActive);

  const { setAdviser } = useAdviser();

  const frozenH =
    originalVesting[DENOM_LIQUID] > 0
      ? new BigNumber(originalVesting[DENOM_LIQUID]).minus(vested[DENOM_LIQUID]).toNumber()
      : 0;

  const eRatio = getERatio(frozenH, balanceHydrogen);

  const liquidH = new BigNumber(balanceHydrogen)
    .minus(frozenH)
    .dp(0, BigNumber.ROUND_FLOOR)
    .toNumber();

  const maxMintTime = getMaxTimeMint(resourcesParams, selected, height);
  const resourceToken = getAmountResource(resourcesParams, selected, height, {
    valueH: value,
    valueDays,
  });

  useEffect(() => {
    const availableSlots =
      SLOTS_MAX - slotsData.filter((slot) => slot.status === 'Unfreezing').length;

    setAdviser(
      <p>
        {slotsData.length === 0 ? (
          <>investmint - temporarily invest tokens to mint tokens</>
        ) : (
          <>you have {availableSlots} slots available for investmint</>
        )}
        <br />
        choose <ValueImg text="milliampere" /> or <ValueImg text="millivolt" /> to mint, the amount
        of <ValueImg text="hydrogen" /> to invest, and the timeframe
      </p>
    );
  }, [setAdviser, slotsData]);

  useEffect(() => {
    if (!queryClient || !addressActive) {
      return;
    }

    queryClient.getBalance(addressActive, DENOM_LIQUID).then((response) => {
      SetBalanceHydrogen(parseFloat(response.amount));
    });
  }, [queryClient, addressActive]);

  useEffect(() => {
    if (!queryClient) {
      return;
    }

    queryClient.resourcesParams().then((response: QueryParamsResponseResources) => {
      setResourcesParams(response.params);
    });

    queryClient.getHeight().then((response) => {
      setHeight(response);
    });
  }, [queryClient]);

  const updateFunc = () => {
    setValue(0);
    setValueDays(1);
    setUpdateAddress((item) => item + 1);
    update();
  };

  const vestedA = useMemo(() => {
    if (originalVesting.milliampere <= 0) {
      return 0;
    }

    const [{ coinDecimals }] = tracesDenom(SelectedState.milliampere);
    const amount = new BigNumber(originalVesting.milliampere).minus(vested.milliampere).toNumber();

    return getDisplayAmount(amount, coinDecimals);
  }, [vested, originalVesting, tracesDenom]);

  const vestedV = useMemo(() => {
    if (originalVesting.millivolt <= 0) {
      return 0;
    }

    const [{ coinDecimals }] = tracesDenom(SelectedState.millivolt);
    const amount = new BigNumber(originalVesting.millivolt).minus(vested.millivolt).toNumber();
    return getDisplayAmount(amount, coinDecimals);
  }, [vested, originalVesting, tracesDenom]);

  const onChangeValue = (amountH: number) => setValue(amountH);

  const onChangeValueDays = (days: number) => setValueDays(days);

  return (
    <>
      <MainContainer>
        <Statistics amount={{ vestedA, vestedV }} />

        <Display>
          <div className={styles.tabs}>
            <Tabs
              options={['milliampere', 'millivolt'].map((item) => {
                return {
                  text: <DenomArr denomValue={item} />,
                  key: item,
                  onClick: () => setSelected(item),
                };
              })}
              selected={selected}
            />
          </div>

          <div className={styles.containerControl}>
            <LiquidBalances amount={{ liquidH, frozenH }} />

            <div className={styles.containerRcSlider}>
              <FormatNumberTokens
                value={resourceToken}
                text={selected}
                styleContainer={{ fontSize: '30px' }}
              />

              <RcSlider
                value={{ amount: value, onChange: onChangeValue }}
                minMax={{ min: 0, max: liquidH }}
                marks={{
                  [liquidH]: returnColorDot(`${formatNumber(liquidH)} H`),
                }}
              />

              <RcSlider
                value={{ amount: valueDays, onChange: onChangeValueDays }}
                minMax={{ min: 1, max: maxMintTime }}
                marks={{
                  1: returnColorDot('1 day'),
                  [maxMintTime]: returnColorDot(`${maxMintTime} days`),
                }}
              />
            </div>

            <ERatio eRatio={eRatio} />

            {value > 0 && (
              <InfoText
                value={{ amount: value, days: valueDays }}
                selected={selected}
                resourceToken={resourceToken}
              />
            )}
          </div>
        </Display>

        {loadingAuthAccounts ? (
          <Dots big />
        ) : (
          <Display noPadding>
            <TableSlots data={slotsData} />
          </Display>
        )}
      </MainContainer>
      <ActionBar
        amountH={value}
        resource={selected}
        valueDays={valueDays}
        resourceAmount={resourceToken}
        updateFnc={updateFunc}
      />
    </>
  );
}

export default Mint;

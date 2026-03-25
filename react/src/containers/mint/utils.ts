import { QueryParamsResponse as QueryParamsResponseResources } from '@cybercongress/cyber-js/build/codec/cyber/resources/v1beta1/query';
import BigNumber from 'bignumber.js';
import { SelectedState } from './types';

const BASE_VESTING_TIME = 86401;
const BASE_MAX_MINT_TIME = 41;
export const SLOTS_MAX = 16;

export const getERatio = (liquidH: number, balanceHydrogen: number) => {
  if (liquidH <= 0 || balanceHydrogen <= 0) {
    return 0;
  }

  return new BigNumber(liquidH)
    .dividedBy(balanceHydrogen)
    .shiftedBy(2)
    .dp(2, BigNumber.ROUND_FLOOR)
    .toNumber();
};

export const getMaxTimeMint = (
  resourcesParams: QueryParamsResponseResources['params'],
  selected: SelectedState,
  height: number
) => {
  if (!resourcesParams || height <= 0) {
    return BASE_MAX_MINT_TIME;
  }

  const { halvingPeriodAmpereBlocks, halvingPeriodVoltBlocks } = resourcesParams;

  const halvingPeriod =
    selected === SelectedState.millivolt ? halvingPeriodVoltBlocks : halvingPeriodAmpereBlocks;

  const halving = new BigNumber(2).pow(
    new BigNumber(height).dividedBy(halvingPeriod).dp(0, BigNumber.ROUND_FLOOR)
  );

  return new BigNumber(new BigNumber(halvingPeriod).multipliedBy(5).multipliedBy(halving))
    .dividedBy(BASE_VESTING_TIME)
    .dp(0, BigNumber.ROUND_FLOOR)
    .toNumber();
};

export const getAmountResource = (
  params: QueryParamsResponseResources['params'],
  selected: SelectedState,
  height: number,
  value: { valueH: number; valueDays: number }
) => {
  const { valueH, valueDays } = value;
  if (!params || valueDays <= 0 || valueH <= 0) {
    return 0;
  }

  const baseLength =
    selected === SelectedState.millivolt
      ? params.baseInvestmintPeriodVolt
      : params.baseInvestmintPeriodAmpere;
  const baseAmount =
    selected === SelectedState.millivolt
      ? params.baseInvestmintAmountVolt?.amount || 0
      : params.baseInvestmintAmountAmpere?.amount || 0;
  const halvingPeriod =
    selected === SelectedState.millivolt
      ? params.halvingPeriodVoltBlocks
      : params.halvingPeriodAmpereBlocks;

  const cycles = new BigNumber(new BigNumber(valueDays).multipliedBy(BASE_VESTING_TIME)).dividedBy(
    baseLength
  );
  const base = new BigNumber(valueH).dividedBy(baseAmount);
  const halving = new BigNumber(0.5).pow(
    new BigNumber(height).dividedBy(halvingPeriod).dp(0, BigNumber.ROUND_FLOOR)
  );

  return new BigNumber(cycles)
    .multipliedBy(base)
    .multipliedBy(halving)
    .dp(0, BigNumber.ROUND_FLOOR)
    .toNumber();
};

import { toUtf8 } from '@cosmjs/encoding';
import { Coin } from '@cosmjs/launchpad';
import { Uint53 } from '@cosmjs/math';
import { CyberClient } from '@cybercongress/cyber-js';
import { QueryLiquidityPoolResponse } from '@cybercongress/cyber-js/build/codec/tendermint/liquidity/v1beta1/query';
import { MsgSwapWithinBatch } from '@cybercongress/cyber-ts/cyber/liquidity/v1beta1/tx';
import BigNumber from 'bignumber.js';
import { QueryValidatorsResponse } from 'cosmjs-types/cosmos/staking/v1beta1/query';
import Long from 'long';
import {
  BASE_DENOM,
  COIN_DECIMALS_RESOURCE,
  DEFAULT_GAS_LIMITS,
  DENOM_LIQUID,
} from 'src/constants/config';
import { calculatePairAmount } from 'src/pages/teleport/swap/utils';
import { ObjKeyValue } from 'src/types/data';
import { authAccounts } from 'src/utils/search/utils';
import { reduceBalances } from 'src/utils/utils';

const coinFunc = (amount: number, denom: string): Coin => {
  return { denom, amount: new BigNumber(amount).toString(10) };
};

const checkDenom = (denom: string) => {
  if (denom === 'millivolt' || denom === 'milliampere') {
    return COIN_DECIMALS_RESOURCE;
  }

  return 0;
};

const MILLISECONDS_IN_SECOND = 1000;
const BASE_VESTING_TIME = 86401;
const BASE_INVESTMINT_AMOUNT_VOLT = 1 * 10 ** 9;
const BASE_INVESTMINT_AMOUNT_AMPERE = 100 * 10 ** 6;
const MAX_SLOTS = 16;
const POOL_TYPE_INDEX = 1;
const SWAP_FEE_RATE = 0.003;

class Soft3MessageFactory {
  private readonly senderAddress: string;

  protected readonly queryClient: CyberClient | undefined;

  static denom() {
    return BASE_DENOM;
  }

  static fee(fee: number | string | undefined = DEFAULT_GAS_LIMITS.toString()) {
    let usedFee;

    if (typeof fee === 'number') {
      usedFee = new BigNumber(DEFAULT_GAS_LIMITS)
        .multipliedBy(fee)
        .dp(0, BigNumber.ROUND_CEIL)
        .toString();
    } else {
      usedFee = fee;
    }

    return {
      amount: [],
      gas: usedFee,
    };
  }

  static liquidDenom() {
    return DENOM_LIQUID;
  }

  constructor(senderAddress: string, queryClient?: CyberClient) {
    this.senderAddress = senderAddress;
    this.queryClient = queryClient;
  }

  private async checkFreeSlotMint() {
    const HALF_MAX_SLOTS = MAX_SLOTS / 2;
    const dataAuthAccounts = await authAccounts(this.senderAddress);

    if (!dataAuthAccounts?.result?.value?.vesting_periods) {
      return true;
    }

    const { vesting_periods: vestingPeriods, start_time: startTime } =
      dataAuthAccounts.result.value;

    if (vestingPeriods.length < HALF_MAX_SLOTS) {
      return true;
    }

    const slotData = [];
    let length = parseFloat(startTime);
    vestingPeriods.forEach((item) => {
      length += parseFloat(item.length);
      const lengthMs = length * MILLISECONDS_IN_SECOND;
      const d = new Date();
      if (lengthMs >= Date.parse(d)) {
        slotData.push(item);
      }
    });

    if (slotData.length >= HALF_MAX_SLOTS) {
      return false;
    }

    return true;
  }

  public async investmint(amount: Coin, resource: 'millivolt' | 'milliampere', length: number) {
    const isMint = await this.checkFreeSlotMint();

    if (!isMint) {
      return undefined;
    }

    const minAmountMint =
      resource === 'milliampere' ? BASE_INVESTMINT_AMOUNT_AMPERE : BASE_INVESTMINT_AMOUNT_VOLT;

    if (new BigNumber(amount.amount).comparedTo(minAmountMint) < 0) {
      return undefined;
    }

    return {
      typeUrl: '/cyber.resources.v1beta1.MsgInvestmint',
      value: {
        neuron: this.senderAddress,
        amount,
        resource,
        length: Long.fromString(new Uint53(length * BASE_VESTING_TIME).toString()),
      },
    };
  }

  public execute(contract: string, msg: Record<string, unknown>, funds?: readonly Coin[]) {
    return {
      typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract',
      value: {
        sender: this.senderAddress,
        contract,
        msg: toUtf8(JSON.stringify(msg)),
        funds: [...(funds || [])],
      },
    };
  }

  public async delegateTokens(amount: Coin) {
    const address = await this.getValidatorAddress();
    return {
      typeUrl: '/cosmos.staking.v1beta1.MsgDelegate',
      value: {
        delegatorAddress: this.senderAddress,
        validatorAddress: address,
        amount,
      },
    };
  }

  private async getValidatorAddress() {
    if (!this.queryClient) {
      return undefined;
    }

    const result = (await this.queryClient.validators(
      'BOND_STATUS_BONDED'
    )) as QueryValidatorsResponse;

    if (!result.validators.length) {
      return undefined;
    }

    const sortedArray = result.validators
      .sort((itemA, itemB) => parseFloat(itemB.tokens) - parseFloat(itemA.tokens))
      .slice(10, result.validators.length - 1);

    const randomIndex = Math.floor(Math.random() * sortedArray.length);

    return sortedArray[randomIndex].operatorAddress;
  }

  private async getBalPool(poolId: number): Promise<ObjKeyValue | undefined> {
    if (!this.queryClient) {
      return undefined;
    }

    const resultPool = (await this.queryClient.pool(poolId)) as QueryLiquidityPoolResponse;

    const { pool } = resultPool;

    if (!pool) {
      return undefined;
    }

    const resultBalances = await this.queryClient.getAllBalances(pool.reserveAccountAddress);

    if (!resultBalances) {
      return undefined;
    }

    const dataReduceBalances = reduceBalances(resultBalances);

    return dataReduceBalances;
  }

  private async getPrice(
    poolId: number,
    token: {
      offerCoin: Coin;
      demandCoinDenom: string;
    }
  ): Promise<string | undefined> {
    const { offerCoin, demandCoinDenom } = token;

    const getBalPool = await this.getBalPool(poolId);

    if (!getBalPool) {
      return undefined;
    }

    const tokenAPoolAmount = getBalPool[offerCoin.denom] || 0;
    const tokenBPoolAmount = getBalPool[demandCoinDenom] || 0;

    if (!tokenAPoolAmount || !tokenBPoolAmount || Number(offerCoin.amount) === 0) {
      return undefined;
    }

    const state = {
      tokenA: offerCoin.denom,
      tokenB: demandCoinDenom,
      tokenAPoolAmount,
      tokenBPoolAmount,
      coinDecimalsA: checkDenom(offerCoin.denom),
      coinDecimalsB: checkDenom(demandCoinDenom),
      isReverse: false,
    };

    const { price } = calculatePairAmount(Number(offerCoin.amount), state);

    const exp = new BigNumber(10).pow(18).toString();

    const convertSwapPrice = new BigNumber(price.toNumber())
      .multipliedBy(exp)
      .dp(0, BigNumber.ROUND_FLOOR)
      .toString(10);

    return convertSwapPrice;
  }

  public async swap(poolId: number, offerCoin: Coin, demandCoinDenom: string) {
    const orderPrice = await this.getPrice(poolId, {
      offerCoin,
      demandCoinDenom,
    });

    if (!orderPrice) {
      return {};
    }

    const amount = new BigNumber(offerCoin.amount)
      .multipliedBy(1 - SWAP_FEE_RATE)
      .dp(0, BigNumber.ROUND_CEIL);

    const offerCoinFee = coinFunc(
      new BigNumber(amount)
        .multipliedBy(SWAP_FEE_RATE)
        .multipliedBy(0.5)
        .dp(0, BigNumber.ROUND_CEIL)
        .toNumber(),

      offerCoin.denom
    );

    return {
      typeUrl: MsgSwapWithinBatch.typeUrl,
      value: {
        swapRequesterAddress: this.senderAddress,
        poolId,
        swapTypeId: POOL_TYPE_INDEX,
        offerCoin: {
          amount: amount.toString(),
          denom: offerCoin.denom,
        },
        demandCoinDenom,
        offerCoinFee,
        orderPrice,
      },
    };
  }
}

export default Soft3MessageFactory;

import { coin } from '@cosmjs/launchpad';
import { CyberClient } from '@cybercongress/cyber-js';
import BigNumber from 'bignumber.js';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';
import { CONTRACT_ADDRESS_GIFT } from './utils';

const LENGTH_INVESTMINT = 1041;
const POOL_ID_H_A = 5;
const POOL_ID_H_V = 6;

const mssgsClaim = async (
  signerInfo: { sender: string; isNanoLedger: boolean },
  releaseMsg: object[],
  availableRelease: number,
  queryClient?: CyberClient,
  onlyDelegate?: boolean
) => {
  const msgsBroadcast = [];
  const { sender } = signerInfo;
  const soft3js = new Soft3MessageFactory(sender, queryClient);

  const amountStake = new BigNumber(availableRelease);

  releaseMsg.forEach((item) => {
    msgsBroadcast.push(soft3js.execute(CONTRACT_ADDRESS_GIFT, item));
  });

  const resultMsgDelegate = await soft3js.delegateTokens(
    coin(amountStake.toString(), Soft3MessageFactory.denom())
  );

  msgsBroadcast.push(resultMsgDelegate);

  if (onlyDelegate) {
    return msgsBroadcast;
  }

  const resultConstructorMintSwap = await constructorMintSwap(soft3js, amountStake);

  msgsBroadcast.push(...resultConstructorMintSwap);

  return msgsBroadcast;
};

const constructorMintSwap = async (soft3js: Soft3MessageFactory, amountStake: BigNumber) => {
  const msgsBroadcast = [];
  const halfAmountStake = amountStake.dividedBy(2).dp(0, BigNumber.ROUND_FLOOR).toString();

  const resultMintA = await soft3js.investmint(
    coin(halfAmountStake, Soft3MessageFactory.liquidDenom()),
    'milliampere',
    LENGTH_INVESTMINT
  );

  if (resultMintA) {
    msgsBroadcast.push(resultMintA);
  } else {
    const offerCoin = coin(halfAmountStake, Soft3MessageFactory.liquidDenom());
    const resultSwapMsg = await soft3js.swap(POOL_ID_H_A, offerCoin, 'milliampere');
    msgsBroadcast.push(resultSwapMsg);
  }

  const resultMintV = await soft3js.investmint(
    coin(halfAmountStake, Soft3MessageFactory.liquidDenom()),
    'millivolt',
    LENGTH_INVESTMINT
  );

  if (resultMintV) {
    msgsBroadcast.push(resultMintV);
  } else {
    const offerCoin = coin(halfAmountStake, Soft3MessageFactory.liquidDenom());
    const resultSwapMsg = await soft3js.swap(POOL_ID_H_V, offerCoin, 'millivolt');
    msgsBroadcast.push(resultSwapMsg);
  }

  return msgsBroadcast;
};

export default mssgsClaim;

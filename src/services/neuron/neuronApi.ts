/* eslint-disable import/no-unused-modules */
import { Coin, OfflineSigner, StdFee } from '@cosmjs/launchpad';
import { SigningCyberClient } from '@cybercongress/cyber-js';
import BigNumber from 'bignumber.js';
import { DEFAULT_GAS_LIMITS } from 'src/constants/config';
import { CONTRACT_ADDRESS_PASSPORT } from 'src/containers/portal/utils';
import { SenseApi } from 'src/contexts/backend/services/senseApi';
import { CyberLinkSimple, NeuronAddress, ParticleCid } from 'src/types/base';
import { getNowUtcNumber } from 'src/utils/date';
import { asyncForEach } from 'src/utils/utils';
import { LinkDto } from '../CozoDb/types/dto';
import Soft3MessageFactory from '../soft.js/api/msgs';
import { throwErrorOrResponse } from './errors';

const defaultFee = {
  amount: [],
  gas: DEFAULT_GAS_LIMITS.toString(),
} as StdFee;

export const sendCyberlink = async (
  neuron: NeuronAddress,
  from: ParticleCid,
  to: ParticleCid,
  {
    senseApi,
    signingClient,
  }: {
    senseApi: SenseApi;
    signingClient: SigningCyberClient;
  },
  fee: StdFee = defaultFee
) => {
  const response = await signingClient.cyberlink(neuron, from, to, fee);
  const { transactionHash } = throwErrorOrResponse(response as any);

  const link = {
    from,
    to,
    transactionHash,
    timestamp: getNowUtcNumber(),
    neuron,
  } as LinkDto;

  // TODO: add from/toparticle to DB ??
  await senseApi?.putCyberlink(link);
  await senseApi?.addCyberlinkLocal(link);

  return transactionHash;
};

export const sendCyberlinkArray = async (
  neuron: NeuronAddress,
  arrLinks: CyberLinkSimple[],
  {
    signingClient,
    senseApi,
  }: {
    senseApi: SenseApi;
    signingClient: SigningCyberClient;
  }
) => {
  const multiplier = new BigNumber(2).multipliedBy(arrLinks.length);

  const cyberlinkMsg = {
    typeUrl: '/cyber.graph.v1beta1.MsgCyberlink',
    value: {
      neuron,
      links: arrLinks,
    },
  };

  const response = await signingClient.signAndBroadcast(
    neuron,
    [cyberlinkMsg],
    Soft3MessageFactory.fee(multiplier.toNumber())
  );

  const result = throwErrorOrResponse(response);

  const { transactionHash } = result;

  const links = arrLinks.map((item) => {
    return {
      from: item.from,
      to: item.to,
      transactionHash,
      timestamp: getNowUtcNumber(),
      neuron,
    } as LinkDto;
  });

  await senseApi?.putCyberlink(links);

  await asyncForEach(links, async (item: LinkDto) => {
    await senseApi?.addCyberlinkLocal(item);
  });

  return transactionHash;
};

export const sendTokensWithMessage = async (
  address: NeuronAddress,
  recipient: string,
  offerCoin: Coin[],
  memo: string | ParticleCid,
  { senseApi, signingClient }: { signingClient: SigningCyberClient; senseApi: SenseApi }
) => {
  const response = await signingClient.sendTokens(address, recipient, offerCoin, 'auto', memo);
  const result = throwErrorOrResponse(response);
  const { transactionHash } = result;

  await senseApi?.addMsgSendAsLocal({
    transactionHash,
    fromAddress: address,
    toAddress: recipient,
    amount: offerCoin,
    memo,
  });

  return transactionHash;
};

export const investmint = async (
  address: NeuronAddress,
  amount: Coin,
  resource: string,
  length: number,
  signingClient: SigningCyberClient
) => {
  const response = await signingClient.investmint(address, amount, resource, length, 'auto');

  const { transactionHash } = throwErrorOrResponse(response);
  return transactionHash;
};

export const updatePassportParticle = async (
  nickname: string,
  particle: ParticleCid,
  {
    signer,
    signingClient,
  }: {
    signer: OfflineSigner;
    signingClient: SigningCyberClient;
  }
) => {
  const [{ address }] = await signer.getAccounts();

  const msgObject = {
    update_particle: {
      nickname,
      particle,
    },
  };
  return signingClient.execute(address, CONTRACT_ADDRESS_PASSPORT, msgObject, 'auto');
};

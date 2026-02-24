import { IbcClient, Link as IbcLink, Logger } from '@confio/relayer/build';
import { RelayedHeights } from '@confio/relayer/build/lib/link';
// import { OfflineDirectSigner, OfflineSigner } from '@cosmjs/proto-signing';
import { GasPrice } from '@cosmjs/stargate';
import { Tendermint34Client } from '@cosmjs/tendermint-rpc';
import { OfflineAminoSigner, OfflineDirectSigner } from '@keplr-wallet/types';
import doCheckAndRelayPrivate from './doCheckAndRelayPrivate';

const sleep = (ms: number) => {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
};
const options = { poll: 4000, maxAgeDest: 86400, maxAgeSrc: 86400 };

async function relay(
  offlineSignerA: OfflineAminoSigner | OfflineDirectSigner,
  offlineSignerB: OfflineAminoSigner | OfflineDirectSigner,
  rpcA: string,
  rpcB: string,
  addrA: string,
  addrB: string,
  GasPriceA: GasPrice,
  GasPriceB: GasPrice,
  cxnA: string,
  cxnB: string,
  /// sender whose packets to relay on chain A
  senderA: string,
  /// sender whose packets to relay on chain B
  senderB: string,
  initHeightA?: number,
  initHeightB?: number,
  logger?: Logger
) {
  // after each line, console.log what it just did. then console log applicable data
  console.debug('relay starting');
  const tmA = await Tendermint34Client.connect(rpcA);
  const tmB = await Tendermint34Client.connect(rpcB);

  if (initHeightA === undefined) {
    const abciA = await tmA.abciInfo();
    console.debug('abciA', abciA);
    initHeightA = abciA.lastBlockHeight! - 50_000;
  }
  if (initHeightB === undefined) {
    const abciB = await tmB.abciInfo();
    console.debug('abciB', abciB);
    initHeightB = abciB.lastBlockHeight! - 50_000;
  }
  const link = await IbcLink.createWithExistingConnections(
    await IbcClient.connectWithSigner(rpcA, offlineSignerA, addrA, {
      // @ts-expect-error
      gasPrice: GasPriceA,
      logger,
    }),
    await IbcClient.connectWithSigner(rpcB, offlineSignerB, addrB, {
      // @ts-expect-error
      gasPrice: GasPriceB,
      logger,
    }),
    cxnA,
    cxnB,
    logger
  );

  let relayFrom: RelayedHeights = {
    packetHeightA: initHeightA,
    packetHeightB: initHeightB,
    ackHeightA: initHeightA,
    ackHeightB: initHeightB,
  };
  let running = true;
  (async () => {
    console.debug('running', running);
    while (running) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const out = await doCheckAndRelayPrivate(
          link,
          relayFrom,
          senderA,
          senderB,
          tmA,
          tmB,
          1000000000,
          1000000000,
          logger!
        );
        relayFrom = out.heights;
        console.debug(out.info);
        // eslint-disable-next-line no-await-in-loop
        await link.updateClientIfStale('A', options.maxAgeDest);
        // eslint-disable-next-line no-await-in-loop
        await link.updateClientIfStale('B', options.maxAgeSrc);
      } catch (error) {
        console.error(`Caught error: `, error);
      }
      console.debug('sleeping for 6 seconds');
      // eslint-disable-next-line no-await-in-loop
      await sleep(options.poll);
    }
  })();

  return {
    stop() {
      console.debug('relay stop');
      running = false;
    },
  };
}

export default relay;

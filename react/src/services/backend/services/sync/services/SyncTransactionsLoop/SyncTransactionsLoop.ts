/* eslint-disable camelcase */

import { isEmpty } from 'lodash';
import {
  combineLatest,
  defer,
  distinctUntilChanged,
  filter,
  from,
  map,
  merge,
  Observable,
} from 'rxjs';
import {
  MessagesByAddressSenseQueryVariables,
  MessagesByAddressSenseWsDocument,
  MessagesByAddressSenseWsSubscription,
} from 'src/generated/graphql';
import { mapIndexerTransactionToEntity } from 'src/services/CozoDb/mapping';
import { SyncStatusDto, TransactionDto } from 'src/services/CozoDb/types/dto';
import { EntryType, SyncQueueJobType } from 'src/services/CozoDb/types/entities';
import { mapWebsocketTxToTransactions } from 'src/services/lcd/utils/mapping';
import {
  createNodeWebsocketObservable,
  getIncomingTransfersQuery,
} from 'src/services/lcd/websocket';
import { QueuePriority } from 'src/services/QueueManager/types';
import { NeuronAddress } from 'src/types/base';
import { asyncIterableBatchProcessor } from 'src/utils/async/iterable';
import { throwIfAborted } from 'src/utils/async/promise';
import { numberToUtcDate } from 'src/utils/date';
import { TRANSACTIONS_BATCH_LIMIT } from '../../../indexer/consts';
import {
  fetchTransactionMessagesCount,
  fetchTransactionsIterable,
  mapMessagesByAddressVariables,
} from '../../../indexer/transactions';
import { createIndexerWebsocket } from '../../../indexer/utils/graphqlClient';
import { SyncServiceParams } from '../../types';
import BaseSyncClient from '../BaseSyncLoop/BaseSyncClient';
import { MAX_DATABASE_PUT_SIZE } from '../consts';
import { ServiceDeps } from '../types';
import { extractCybelinksFromTransaction } from '../utils/links';
import { syncMyChats } from './services/chat';

type DataStreamResult = {
  source: 'indexer' | 'node';
  transactions: TransactionDto[];
};

class SyncTransactionsLoop extends BaseSyncClient {
  protected createIsInitializedObserver(deps: ServiceDeps) {
    const isInitialized$ = combineLatest([
      deps.dbInstance$,
      deps.params$!.pipe(
        map((params) => params.myAddress),
        distinctUntilChanged()
      ),
      this.particlesResolver!.isInitialized$,
    ]).pipe(
      map(
        ([dbInstance, myAddress, syncQueueInitialized]) =>
          !!dbInstance && !!syncQueueInitialized && !!myAddress
      )
    );

    return isInitialized$;
  }

  // eslint-disable-next-line class-methods-use-this
  protected createClientObservable(timestampFrom: number): Observable<DataStreamResult> {
    const { myAddress } = this.params;
    this.cyblogCh.info(
      `>>> ${this.name} subscribe ${myAddress} from ${numberToUtcDate(timestampFrom)}`
    );

    const variables = mapMessagesByAddressVariables({
      neuron: myAddress!,
      timestampFrom,
      types: [],
      orderDirection: 'desc',
      limit: 100,
    }) as MessagesByAddressSenseQueryVariables;

    const indexerObservable$ = createIndexerWebsocket<MessagesByAddressSenseWsSubscription>(
      MessagesByAddressSenseWsDocument,
      variables
    ).pipe(
      map((response: MessagesByAddressSenseWsSubscription) => {
        return {
          source: 'indexer',
          transactions: response.messages_by_address.map((i) =>
            mapIndexerTransactionToEntity(myAddress!, i)
          ),
        };
      })
    );

    const nodeObservample$ = createNodeWebsocketObservable(
      myAddress!,
      getIncomingTransfersQuery(myAddress!),
      (message, ctx) => this.cyblogCh.info(message, { unit: 'node-ws', ...ctx })
    ).pipe(
      filter((data) => !isEmpty(data)),
      map((data) => {
        return {
          source: 'node',
          transactions: mapWebsocketTxToTransactions(myAddress!, data),
        };
      })
    );

    return merge(indexerObservable$, nodeObservample$) as Observable<DataStreamResult>;
  }

  protected createInitObservable() {
    return defer(() => from(this.initSync()));
    // return from(this.initSync());
  }

  public async initSync() {
    const { myAddress } = this.params;
    const { signal } = this.abortController;
    const syncItem = await this.db!.getSyncStatus(myAddress!, myAddress!);

    const lastTransactionTimestamp = await this.syncTransactions(myAddress!, myAddress!, syncItem);

    this.statusApi.sendStatus('in-progress', `sync my chats`);
    const syncStatusItems = await syncMyChats(
      this.db!,
      myAddress!,
      syncItem.timestampUpdate,
      signal
    );

    this.channelApi.postSenseUpdate(syncStatusItems);
    this.statusApi.sendStatus('active');

    return lastTransactionTimestamp;
  }

  protected async onUpdate({ source, transactions }: DataStreamResult, params: SyncServiceParams) {
    const { myAddress } = params;
    const { signal } = this.abortController;
    if (transactions.length === 0) {
      this.cyblogCh.info(`>>> ${this.name} ${myAddress} recived 0 updates `);
      return;
    }
    const syncItem = await this.db!.getSyncStatus(myAddress!, myAddress!);

    await this.processBatchTransactions(myAddress!, myAddress!, transactions, syncItem, source);

    this.statusApi.sendStatus('in-progress', `sync my chats`);
    const syncStatusItems = await syncMyChats(
      this.db!,
      myAddress!,
      syncItem.timestampUpdate,
      signal,
      source !== 'node'
    );

    this.channelApi.postSenseUpdate(syncStatusItems);
    this.statusApi.sendStatus('listen');
  }

  public async processBatchTransactions(
    myAddress: NeuronAddress,
    address: NeuronAddress,
    transactions: TransactionDto[],
    { timestampRead, unreadCount, timestampUpdate }: SyncStatusDto,
    source: DataStreamResult['source']
  ) {
    const { signal } = this.abortController;

    // node transaction is limited by incoming messages,
    // to prevent missing of other msg types let's avoid to change ts
    const shouldUpdateTimestamp = source !== 'node';

    this.cyblogCh.info(
      `   syncTransactions - process ${address}[${source}],  count: ${
        transactions.length
      }, from: ${transactions.at(0)?.timestamp}, to: ${transactions.at(-1)?.timestamp}`
    );

    // save transaction
    await throwIfAborted(this.db!.putTransactions, signal)(transactions);

    // save links
    this.syncLinks(transactions, signal);

    const {
      hash,
      index,

      timestamp,
    } = transactions.at(-1)!;

    const lastTimestampFrom = timestamp;

    // Update transaction sync items
    const newSyncItem = {
      ownerId: myAddress,
      entryType: EntryType.transactions,
      id: address,
      timestampUpdate: shouldUpdateTimestamp ? lastTimestampFrom : timestampUpdate!,
      unreadCount: unreadCount! + transactions.length,
      timestampRead: timestampRead || 0,
      disabled: false,
      meta: {
        transactionHash: hash,
        index,
      },
    };

    await throwIfAborted(this.db!.putSyncStatus, signal)(newSyncItem);

    return lastTimestampFrom;
  }

  public async syncTransactions(
    myAddress: NeuronAddress,
    address: NeuronAddress,
    syncItem: SyncStatusDto
  ) {
    const { unreadCount, timestampUpdate } = syncItem;
    const timestampFrom = timestampUpdate + 1; // ofsset + 1 to fix milliseconds precision bug

    this.statusApi.sendStatus('estimating');

    const totalMessageCount = await fetchTransactionMessagesCount(
      address,
      timestampFrom,
      this.abortController!.signal
    );

    this.cyblogCh.info(
      `>>> syncTransactions - start ${address},  count: ${totalMessageCount}, from: ${timestampFrom}`
    );

    if (totalMessageCount === 0) {
      return timestampFrom;
    }

    this.statusApi.sendStatus(
      'in-progress',
      `sync ${address}...`,
      this.progressTracker.start(Math.ceil(totalMessageCount / TRANSACTIONS_BATCH_LIMIT))
    );

    const transactionsAsyncIterable = fetchTransactionsIterable({
      neuron: address,
      timestampFrom,
      types: [], // SENSE_TRANSACTIONS,
      orderDirection: 'asc',
      limit: TRANSACTIONS_BATCH_LIMIT,
      abortSignal: this.abortController?.signal,
    });

    let transactionCount = 0;
    let lastTimestampFrom = timestampFrom;

    // eslint-disable-next-line no-restricted-syntax
    for await (const batch of transactionsAsyncIterable) {
      this.statusApi.sendStatus(
        'in-progress',
        `sync ${address}...`,
        this.progressTracker.trackProgress(1)
      );

      transactionCount += batch.length;

      const transactions = batch.map((i) => mapIndexerTransactionToEntity(address, i));

      lastTimestampFrom = await this.processBatchTransactions(
        myAddress,
        address,
        transactions,
        {
          ...syncItem,
          unreadCount: unreadCount + transactionCount,
        },
        'indexer'
      );
    }

    return lastTimestampFrom;
  }

  private async syncLinks(batch: TransactionDto[], signal: AbortSignal) {
    const { tweets, particlesFound, links } = extractCybelinksFromTransaction(batch);
    if (links.length > 0) {
      await asyncIterableBatchProcessor(
        links,
        (links) => throwIfAborted(this.db!.putCyberlinks, signal)(links),
        MAX_DATABASE_PUT_SIZE
      );
    }

    const tweetParticles = Object.keys(tweets);

    const nonTweetParticles = particlesFound.filter((cid) => !tweetParticles.includes(cid));

    // pre-resolve 'tweets' particles
    await this.particlesResolver!.enqueueBatch(
      tweetParticles,
      SyncQueueJobType.particle,
      QueuePriority.HIGH
    );

    // pre-resolve all the rest particles
    if (nonTweetParticles.length > 0) {
      await this.particlesResolver!.enqueueBatch(
        nonTweetParticles,
        SyncQueueJobType.particle,
        QueuePriority.LOW
      );
    }
  }
}

export default SyncTransactionsLoop;

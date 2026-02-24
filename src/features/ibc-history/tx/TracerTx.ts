import { toBase64, toHex } from 'src/utils/encoding';

import { TxEventMap, WsReadyState } from './types';

type Listeners = {
  [K in keyof TxEventMap]?: TxEventMap[K][];
};

/**
 * TxTracer is almost same with the `TendermintTxTracer` in the @keplr-wallet/cosmos library.
 * Changes for some mistake on the original `TendermintTxTracer` and this would be remove if the changes are merged to the original library.
 */
class TxTracer {
  protected ws!: WebSocket;

  protected newBlockSubscribes: {
    handler: (block: any) => void;
  }[] = [];

  // Key is "id" for jsonrpc
  protected txSubscribes: Map<
    number,
    {
      params: Record<string, string | number | boolean>;
      resolver: (data?: unknown) => void;
      rejector: (e: Error) => void;
    }
  > = new Map();

  // Key is "id" for jsonrpc
  protected pendingQueries: Map<
    number,
    {
      method: string;
      params: Record<string, string | number | boolean>;
      resolver: (data?: unknown) => void;
      rejector: (e: Error) => void;
    }
  > = new Map();

  protected listeners: Listeners = {};

  constructor(
    protected readonly url: string,
    protected readonly wsEndpoint: string,
    protected readonly options: {
      wsObject?: new (url: string, protocols?: string | string[]) => WebSocket;
    } = {}
  ) {
    this.open();
  }

  protected getWsEndpoint(): string {
    let { url } = this;
    if (url.startsWith('http')) {
      url = url.replace('http', 'ws');
    }
    if (!url.endsWith(this.wsEndpoint)) {
      const wsEndpoint = this.wsEndpoint.startsWith('/') ? this.wsEndpoint : `/${this.wsEndpoint}`;

      url = url.endsWith('/') ? url + wsEndpoint.slice(1) : url + wsEndpoint;
    }

    return url;
  }

  open() {
    this.ws = this.options.wsObject
      ? new this.options.wsObject(this.getWsEndpoint())
      : new WebSocket(this.getWsEndpoint());
    this.ws.onopen = this.onOpen;
    this.ws.onmessage = this.onMessage;
    this.ws.onclose = this.onClose;
    this.ws.onerror = this.onError;
  }

  close() {
    this.ws.close();
  }

  get numberOfSubscriberOrPendingQuery(): number {
    return this.newBlockSubscribes.length + this.txSubscribes.size + this.pendingQueries.size;
  }

  get readyState(): WsReadyState {
    switch (this.ws.readyState) {
      case 0:
        return WsReadyState.CONNECTING;
      case 1:
        return WsReadyState.OPEN;
      case 2:
        return WsReadyState.CLOSING;
      case 3:
        return WsReadyState.CLOSED;
      default:
        return WsReadyState.NONE;
    }
  }

  addEventListener<T extends keyof TxEventMap>(type: T, listener: TxEventMap[T]) {
    if (!this.listeners[type]) {
      this.listeners[type] = [];
    }

    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-expect-error
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    this.listeners[type]!.push(listener);
  }

  protected readonly onOpen = (e: Event) => {
    if (this.newBlockSubscribes.length > 0) {
      this.sendSubscribeBlockRpc();
    }

    for (const [id, tx] of this.txSubscribes) {
      this.sendSubscribeTxRpc(id, tx.params);
    }

    for (const [id, query] of this.pendingQueries) {
      this.sendQueryRpc(id, query.method, query.params);
    }

    for (const listener of this.listeners.open ?? []) {
      listener(e);
    }
  };

  protected readonly onMessage = (e: MessageEvent) => {
    for (const listener of this.listeners.message ?? []) {
      listener(e);
    }

    if (e.data) {
      try {
        const obj = JSON.parse(e.data);

        if (obj?.id) {
          if (this.pendingQueries.has(obj.id)) {
            if (obj.error) {
              // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
              this.pendingQueries
                .get(obj.id)!
                .rejector(new Error(obj.error.data || obj.error.message));
            } else {
              // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
              this.pendingQueries.get(obj.id)!.resolver(obj.result);
            }

            this.pendingQueries.delete(obj.id);
          }
        }

        if (obj?.result?.data?.type === 'tendermint/event/NewBlock') {
          for (const handler of this.newBlockSubscribes) {
            handler.handler(obj.result.data.value);
          }
        }

        if (obj?.result?.data?.type === 'tendermint/event/Tx') {
          if (obj?.id) {
            if (this.txSubscribes.has(obj.id)) {
              if (obj.error) {
                // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
                this.txSubscribes
                  .get(obj.id)!
                  .rejector(new Error(obj.error.data || obj.error.message));
              } else {
                // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
                this.txSubscribes.get(obj.id)!.resolver(obj.result.data.value.TxResult.result);
              }

              this.txSubscribes.delete(obj.id);
            }
          }
        }
      } catch (e: any) {
        console.error(
          `Tendermint websocket jsonrpc response is not JSON: ${e.message || e.toString()}`
        );
      }
    }
  };

  protected readonly onClose = (e: CloseEvent) => {
    // Reject all pending queries and subscriptions when WebSocket closes
    const error = new Error(`WebSocket closed: ${e.reason || 'connection lost'}`);

    for (const [_id, query] of this.pendingQueries) {
      query.rejector(error);
    }
    this.pendingQueries.clear();

    for (const [_id, sub] of this.txSubscribes) {
      sub.rejector(error);
    }
    this.txSubscribes.clear();

    for (const listener of this.listeners.close ?? []) {
      listener(e);
    }
  };

  protected readonly onError = (_e: Event) => {
    // Reject all pending queries and subscriptions on WebSocket error
    const error = new Error('WebSocket error');

    for (const [_id, query] of this.pendingQueries) {
      query.rejector(error);
    }
    this.pendingQueries.clear();

    for (const [_id, sub] of this.txSubscribes) {
      sub.rejector(error);
    }
    this.txSubscribes.clear();
  };

  /**
   * SubscribeBlock receives the handler for the block.
   * The handelrs shares the subscription of block.
   * @param handler
   * @return unsubscriber
   */
  subscribeBlock(handler: (block: any) => void): () => void {
    this.newBlockSubscribes.push({
      handler,
    });

    if (this.newBlockSubscribes.length === 1) {
      this.sendSubscribeBlockRpc();
    }

    return () => {
      this.newBlockSubscribes = this.newBlockSubscribes.filter((s) => s.handler !== handler);
    };
  }

  protected sendSubscribeBlockRpc(): void {
    if (this.readyState === WsReadyState.OPEN) {
      this.ws.send(
        JSON.stringify({
          jsonrpc: '2.0',
          method: 'subscribe',
          params: { query: "tm.event='NewBlock'" },
          id: 1,
        })
      );
    }
  }

  // Query the tx and subscribe the tx.
  traceTx(
    query: Uint8Array | Record<string, string | number | boolean>,
    {
      timeoutMs = 120000,
      connectionTimeoutMs = 15000,
    }: { timeoutMs?: number; connectionTimeoutMs?: number } = {}
  ): Promise<any> {
    const timeoutPromise = new Promise<never>((_, reject) => {
      setTimeout(() => reject(new Error(`traceTx timed out after ${timeoutMs}ms`)), timeoutMs);
    });

    const connectionPromise =
      this.readyState !== WsReadyState.OPEN
        ? new Promise<void>((resolve, reject) => {
            const connectionTimer = setTimeout(() => {
              reject(new Error(`WebSocket connection timed out after ${connectionTimeoutMs}ms`));
            }, connectionTimeoutMs);

            this.addEventListener('open', () => {
              clearTimeout(connectionTimer);
              resolve();
            });
          })
        : Promise.resolve();

    const tracePromise = connectionPromise.then(
      () =>
        new Promise<any>((resolve) => {
          this.queryTx(query)
            .then((result) => {
              if (query instanceof Uint8Array) {
                resolve(result);
                return;
              }

              if (result?.total_count !== '0') {
                resolve(result);
              }
            })
            .catch(() => {
              // noop
            });

          this.subscribeTx(query).then(resolve);
        })
    );

    return Promise.race([tracePromise, timeoutPromise]);
  }

  subscribeTx(query: Uint8Array | Record<string, string | number | boolean>): Promise<any> {
    if (query instanceof Uint8Array) {
      const id = this.createRandomId();

      const params = {
        query: `tm.event='Tx' AND tx.hash='${toHex(query).toUpperCase()}'`,
      };

      return new Promise<unknown>((resolve, reject) => {
        this.txSubscribes.set(id, {
          params,
          resolver: resolve,
          rejector: reject,
        });

        this.sendSubscribeTxRpc(id, params);
      });
    }
    const id = this.createRandomId();

    const params = {
      query: `tm.event='Tx' AND ${Object.keys(query)
        .map((key) => {
          return {
            key,
            value: query[key],
          };
        })
        .map((obj) => {
          return `${obj.key}=${typeof obj.value === 'string' ? `'${obj.value}'` : obj.value}`;
        })
        .join(' AND ')}`,
      page: '1',
      per_page: '1',
      order_by: 'desc',
    };

    return new Promise<unknown>((resolve, reject) => {
      this.txSubscribes.set(id, {
        params,
        resolver: resolve,
        rejector: reject,
      });

      this.sendSubscribeTxRpc(id, params);
    });
  }

  protected sendSubscribeTxRpc(
    id: number,
    params: Record<string, string | number | boolean>
  ): void {
    if (this.readyState === WsReadyState.OPEN) {
      this.ws.send(
        JSON.stringify({
          jsonrpc: '2.0',
          method: 'subscribe',
          params,
          id,
        })
      );
    }
  }

  queryTx(query: Uint8Array | Record<string, string | number | boolean>): Promise<any> {
    if (query instanceof Uint8Array) {
      return this.query('tx', {
        hash: toBase64(query),
        prove: false,
      });
    }
    const params = {
      query: Object.keys(query)
        .map((key) => {
          return {
            key,
            value: query[key],
          };
        })
        .map((obj) => {
          return `${obj.key}=${typeof obj.value === 'string' ? `'${obj.value}'` : obj.value}`;
        })
        .join(' AND '),
      page: '1',
      per_page: '1',
      order_by: 'desc',
    };

    return this.query('tx_search', params);
  }

  protected query(method: string, params: Record<string, string | number | boolean>): Promise<any> {
    const id = this.createRandomId();

    return new Promise<unknown>((resolve, reject) => {
      this.pendingQueries.set(id, {
        method,
        params,
        resolver: resolve,
        rejector: reject,
      });

      this.sendQueryRpc(id, method, params);
    });
  }

  protected sendQueryRpc(
    id: number,
    method: string,
    params: Record<string, string | number | boolean>
  ) {
    if (this.readyState === WsReadyState.OPEN) {
      this.ws.send(
        JSON.stringify({
          jsonrpc: '2.0',
          method,
          params,
          id,
        })
      );
    }
  }

  protected createRandomId(): number {
    return parseInt(
      Array.from({ length: 6 })
        .map(() => Math.floor(Math.random() * 100))
        .join(''),
      10
    );
  }
}

export default TxTracer;

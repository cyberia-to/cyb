import Axios, { AxiosInstance, AxiosRequestConfig } from 'axios';

/** Polls a `/status` endpoint on a given Axios RPC config and publishes to an arbitrary set of subsribers. */
class PollingStatusSubscription {
  protected readonly rpcInstance: AxiosInstance;

  protected _subscriptionCount = 0;

  protected _handlers: ((data: any) => void)[] = [];

  constructor(
    protected readonly rpc: string,
    protected readonly rpcConfig?: AxiosRequestConfig
  ) {
    this.rpcInstance = Axios.create({
      ...{
        baseURL: rpc,
      },
      ...rpcConfig,
    });
  }

  get subscriptionCount(): number {
    return this._subscriptionCount;
  }

  /**
   * @param handler
   * @return unsubscriber
   */
  subscribe(handler: (data: any) => void): () => void {
    this._handlers.push(handler);

    this.increaseSubscriptionCount();

    return () => {
      this._handlers = this._handlers.filter((h) => h !== handler);
      this.decreaseSubscriptionCount();
    };
  }

  protected async startSubscription() {
    const BASE_INTERVAL = 7500;
    const MAX_INTERVAL = 60000;
    const CIRCUIT_BREAKER_THRESHOLD = 10;

    let currentInterval = BASE_INTERVAL;
    let consecutiveErrors = 0;

    while (this._subscriptionCount > 0) {
      // eslint-disable-next-line no-await-in-loop
      await new Promise((resolve) => {
        setTimeout(resolve, currentInterval);
      });

      // Circuit breaker: pause longer after many consecutive failures
      if (consecutiveErrors >= CIRCUIT_BREAKER_THRESHOLD) {
        console.warn(
          `PollingStatusSubscription: circuit breaker triggered after ${consecutiveErrors} failures, pausing 5 min`
        );
        // eslint-disable-next-line no-await-in-loop
        await new Promise((resolve) => {
          setTimeout(resolve, 300000);
        });
        consecutiveErrors = 0;
        currentInterval = BASE_INTERVAL;
      }

      try {
        // eslint-disable-next-line no-await-in-loop
        const response = await this.rpcInstance.get('/status');
        if (response.status === 200) {
          this._handlers.forEach((handler) => handler(response.data));
          // Reset on success
          consecutiveErrors = 0;
          currentInterval = BASE_INTERVAL;
        }
      } catch (e: any) {
        consecutiveErrors += 1;
        // Exponential backoff: double interval up to max
        currentInterval = Math.min(currentInterval * 2, MAX_INTERVAL);
        console.error(
          `Failed to fetch /status (attempt ${consecutiveErrors}): ${e?.toString()}`
        );
      }
    }
  }

  protected increaseSubscriptionCount() {
    this._subscriptionCount++;

    if (this._subscriptionCount === 1) {
      // No need to await
      this.startSubscription();
    }
  }

  protected decreaseSubscriptionCount() {
    this._subscriptionCount--;
  }
}

export default PollingStatusSubscription;

export class QueueItemTimeoutError extends Error {
  constructor(timeoutMs: number) {
    super(`Timeout after ${timeoutMs}`);
    Object.setPrototypeOf(this, QueueItemTimeoutError.prototype);
  }
}

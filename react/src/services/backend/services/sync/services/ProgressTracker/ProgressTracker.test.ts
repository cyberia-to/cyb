import { ProgressTracker } from './ProgressTracker'; // Adjust the import path as necessary

// eslint-disable-next-line no-promise-executor-return
const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

describe('ProgressTracker', () => {
  let progressTracker: ProgressTracker;
  let mockCallback: jest.Mock;

  beforeEach(() => {
    mockCallback = jest.fn();
    progressTracker = new ProgressTracker(mockCallback);
  });

  test('should call onProgressUpdate with correct parameters after multiple trackProgress calls', async () => {
    const totalRequests = 10;
    progressTracker.start(totalRequests);

    for (let i = 0; i < totalRequests; i++) {
      // eslint-disable-next-line no-await-in-loop
      await sleep(100); // simulate a delay
      progressTracker.trackProgress(1);
    }

    expect(mockCallback).toHaveBeenCalledTimes(totalRequests);

    const firstCall = mockCallback.mock.calls[0][0];
    expect(firstCall.estimatedTime).toBeGreaterThanOrEqual(100);

    // Check the last call to ensure it reflects the state after all requests
    const lastCall = mockCallback.mock.calls.at(-1)[0];
    expect(lastCall.totalCount).toBe(totalRequests);
    expect(lastCall.completeCount).toBe(totalRequests);
    expect(lastCall.estimatedTime).toBeGreaterThanOrEqual(100);
  });
});

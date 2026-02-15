import QueueManager from '../QueueManager';
import { of } from 'rxjs';
import { CybIpfsNode } from '../../ipfs/types';
import { QueueStrategy } from '../QueueStrategy';
import { valuesExpected } from 'src/utils/test-utils/test-utils';
import { fetchIpfsContent } from 'src/services/ipfs/utils/utils-ipfs';

// const mockTimeout = () => (source) => {
//   console.log('TIMEOUT');
//   return source;
// };

// Replace the real timeout with the mock
// rxjsOperators.timeout = mockTimeout;

jest.mock('rxjs', () => {
  const originalOperators = jest.requireActual('rxjs');
  return {
    ...originalOperators,
    debounceTime: jest.fn().mockImplementation(() => (source$) => source$),
  };
});

jest.mock('../../ipfs/utils/ipfsCacheDb');

jest.mock('src/services/ipfs/utils/cluster.ts', () => ({ add: jest.fn() }));
jest.mock('src/services/backend/channels/BroadcastChannelSender');

jest.mock('src/services/ipfs/utils/utils-ipfs.ts', () => ({
  ipfsCacheDb: jest.fn(),
  cyberCluster: jest.fn(),
  fetchIpfsContent: jest.fn(),
  addContenToIpfs: jest.fn(),
}));

const TIMEOUT_MS = 300;

const queueStrategy = new QueueStrategy(
  {
    db: { timeout: TIMEOUT_MS, maxConcurrentExecutions: 2 },
    node: { timeout: TIMEOUT_MS, maxConcurrentExecutions: 2 },
    gateway: { timeout: TIMEOUT_MS, maxConcurrentExecutions: 2 },
  },
  ['db', 'node', 'gateway']
);

const cid1 = 'cid1';
const cid2 = 'cid1';
const cid3 = 'cid1';

const nextTick = () => {
  return new Promise((resolve) => {
    setTimeout(resolve, 0);
  });
};

function wrapPromiseWithSignal(
  promise: Promise<string>,
  signal?: AbortSignal
): Promise<string> {
  return new Promise((resolve, reject) => {
    promise.then((result) => {
      resolve(result);
    });

    signal?.addEventListener('abort', (e) => {
      // @ts-ignore
      if (e?.target?.reason !== 'timeout') {
        reject(new DOMException('canceled', 'AbortError'));
      }
    });
  });
}

const getPromise = (
  result = 'result',
  timeout = 500,
  signal?: AbortSignal
): Promise<string> =>
  wrapPromiseWithSignal(
    new Promise<string>((resolve) => {
      setTimeout(() => resolve(`result ${result}`), timeout);
    }),
    signal
  );

const mockNode: jest.Mocked<CybIpfsNode> = {
  nodeType: 'external',
  reconnectToSwarm: jest.fn(),
};
jest.useFakeTimers();

describe('QueueManager without timers', () => {
  let queueManager: QueueManager;

  beforeEach(() => {
    // setup QueueManager instance before each test

    queueManager = new QueueManager(of(mockNode), {
      strategy: queueStrategy,
      queueDebounceMs: 0,
    });
  });

  afterEach(() => {
    jest.clearAllTimers();
  });

  test('should enqueue item, try to resolve all sources and return not_found', (done) => {
    (fetchIpfsContent as jest.Mock).mockResolvedValue(undefined);
    const responsesExpected = valuesExpected([
      ['pending', 'db'],
      ['executing', 'db'],
      ['error', 'db'],
      ['pending', 'node'],
      ['executing', 'node'],
      ['error', 'node'],
      ['pending', 'gateway'],
      ['executing', 'gateway'],
      ['error', 'gateway'],
      ['not_found', 'gateway'],
    ]);

    queueManager.enqueue(cid1, (cid, status, source) => {
      expect(cid).toBe(cid1);

      const [statusExpected, sourceExpected] = responsesExpected.next().value;
      expect(status).toBe(statusExpected);
      expect(source).toBe(sourceExpected);
      jest.runOnlyPendingTimers();
      if (statusExpected === 'not_found') {
        done();
      }
    });
  });
});

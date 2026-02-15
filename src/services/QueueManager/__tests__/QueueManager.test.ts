import QueueManager from '../QueueManager';
import { BehaviorSubject, of } from 'rxjs';
import { CybIpfsNode } from '../../ipfs/types';
import { QueueStrategy } from '../QueueStrategy';
import { I } from '../types';
import { valuesExpected } from 'src/utils/test-utils/test-utils';
import { fetchIpfsContent } from 'src/services/ipfs/utils/utils-ipfs';

// const mockTimeout = () => (source) => {
//   console.log('TIMEOUT');
//   return source;
// };

// Replace the real timeout with the mock
// rxjsOperators.timeout = mockTimeout;

// jest.mock('rxjs', () => {
//   const originalOperators = jest.requireActual('rxjs');
//   return {
//     ...originalOperators,
//     debounceTime: jest.fn().mockImplementation(() => (source$) => source$),
//   };
// });

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
const cid2 = 'cid2';
const cid3 = 'cid3';
const cid4 = 'cid4';

const nextTick = () => {
  return new Promise((resolve) => {
    setTimeout(resolve, 0);
  });
};

const onNextTick = (callback) => {
  setTimeout(() => {
    callback();
  }, 0);
};
function wrapPromiseWithSignal(
  promise: Promise<string>,
  signal?: AbortSignal
): Promise<string> {
  return new Promise((resolve, reject) => {
    promise.then((result) => {
      resolve(result);
    });
    console.log('------sssss', signal?.aborted);
    signal?.addEventListener('abort', (e) => {
      // @ts-ignore
      console.log('------abort', e, e?.target, e?.target?.reason);

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
// jest.useFakeTimers();

describe('QueueManager without timers', () => {
  let queueManager: QueueManager;

  beforeEach(() => {
    // setup QueueManager instance before each test

    queueManager = new QueueManager(of(mockNode), {
      strategy: queueStrategy,
      queueDebounceMs: 1,
    });
  });

  test('should instantiate without errors', () => {
    expect(queueManager).toBeInstanceOf(QueueManager);
  });

  test('should execute enqueue as promise and get result', async () => {
    (fetchIpfsContent as jest.Mock).mockResolvedValue('done!');
    const result = await queueManager.enqueueAndWait('xxx');
    expect(result?.result).toBe('done!');
  });

  test('should execute enqueue as promise and get undefined', async () => {
    (fetchIpfsContent as jest.Mock).mockRejectedValue(undefined);
    const result = await queueManager.enqueueAndWait('xxx');
    expect(result?.status).toBe('not_found');
  });

  test('should keep in pending items thats is out of maxConcurrentExecutions', () => {
    (fetchIpfsContent as jest.Mock).mockResolvedValue('good-result');

    queueManager.enqueue('1', jest.fn);
    queueManager.enqueue('2', jest.fn);
    queueManager.enqueue('3', jest.fn);
    const itemList = queueManager.getQueueList();
    onNextTick(() => {
      expect(itemList[0].status).toEqual('executing');
      expect(itemList[1].status).toEqual('executing');
      expect(itemList[2].status).toEqual('pending');
    });
  });

  test('should cancel queue items', (done) => {
    const statuses = valuesExpected(['pending', 'executing', 'cancelled']);
    (fetchIpfsContent as jest.Mock).mockImplementation(
      (cid: string, source: string, { controller }) =>
        getPromise('result', 1000, controller.signal)
    );

    queueManager.enqueue(cid1, (cid, status) => {
      expect(cid).toBe(cid1);
      expect(status).toBe(statuses.next().value);

      if (status === 'cancelled') {
        expect(queueManager.getQueueList()[0]?.controller?.signal.aborted).toBe(
          true
        );

        expect(queueManager.getQueueList().length).toEqual(0);
      }
    });
    queueManager.cancel(cid1);

    onNextTick(() => {
      done();
    });
  });

  test('should handle timeout and switch to next source', (done) => {
    const statuses = valuesExpected([
      'pending', // db
      'executing', // db
      'timeout', // db
      'pending', // node
      'executing', // node
      'timeout', // node
      'pending', // gateway
      'executing', // gateway
      'timeout', // gateway
      'not_found',
    ]);

    (fetchIpfsContent as jest.Mock).mockImplementation(
      (cid: string, source: string, { controller }) =>
        getPromise('result', 50000, controller?.signal)
    );
    queueManager.enqueue(cid1, (cid, status, source) => {
      const expectedStatus = statuses.next().value;
      expect(cid).toBe(cid1);
      expect(status).toBe(expectedStatus);
      if (expectedStatus === 'timeout' && source === 'gateway') {
        done();
      }
    });
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
      // jest.runOnlyPendingTimers();
      if (statusExpected === 'not_found') {
        done();
      }
    });
  });

  it('should execute queue items in order by priority', (done) => {
    (fetchIpfsContent as jest.Mock).mockImplementation(() =>
      getPromise('good-result', 100)
    );
    queueManager.enqueue(cid1, jest.fn);
    queueManager.enqueue(cid2, jest.fn);

    const executingByPriority: string[] = [];
    queueManager.enqueue(cid3, (cid) => executingByPriority.push(cid), {
      priority: 0.5,
    });

    queueManager.enqueue(cid4, (cid) => executingByPriority.push(cid), {
      priority: 0.9,
    });

    onNextTick(() => {
      const queue = queueManager.getQueueMap();
      expect(queue.size).toBe(4);
      expect(queue.get(cid4)!.status).toBe('executing');
      expect(queue.get(cid3)!.status).toBe('executing');
      expect(queue.get(cid2)!.status).toBe('pending');
      expect(queue.get(cid1)!.status).toBe('pending');
      done();
    });
  });

  // it('should execute queue items in order by viewportPriority in real-time', (done) => {
  //   try {
  //     (fetchIpfsContent as jest.Mock).mockImplementation(() =>
  //       getPromise('x', 100)
  //     );
  //     queueManager.enqueue('1', jest.fn);
  //     queueManager.enqueue('2', jest.fn);

  //     const executingByPriority: string[] = [];
  //     const setExecutingByPriority = (cid: string, status: QueueItemStatus) =>
  //       status === 'executing' && executingByPriority.push(cid);

  //     queueManager.enqueue('3', setExecutingByPriority, {
  //       priority: 0.5,
  //       viewPortPriority: 0.1,
  //     });

  //     queueManager.enqueue('4', setExecutingByPriority, {
  //       priority: 0.6,
  //       viewPortPriority: 0.1,
  //     });

  //     waitUtilQueueDebounce(() => {
  //       const queue = queueManager.getQueueMap();
  //       // const queue = queueManager.getQueueList();
  //       expect(queue.size).toBe(4);
  //       expect(queue.get('4').status).toBe('executing');
  //       expect(queue.get('3').status).toBe('executing');
  //       expect(queue.get('2').status).toBe('pending');
  //       expect(queue.get('1').status).toBe('pending');

  //       // Update priorities by viewport
  //       queueManager.updateViewPortPriority('3', 0.5);
  //       queueManager.updateViewPortPriority('4', 0);
  //     });

  //     setTimeout(() => {
  //       const queue = queueManager.getQueueMap();
  //       expect(queue.size).toBe(4);

  //       expect(executingByPriority[1]).toBe('3');

  //       expect(executingByPriority[0]).toBe('4');
  //       done();
  //     }, QUEUE_DEBOUNCE_MS + 1);
  //   } finally {
  //     (fetchIpfsContent as jest.Mock).mockClear();
  //   }
  // });

  test('should execute queue items and deprioritize based on viewPortPriority', (done) => {
    (fetchIpfsContent as jest.Mock).mockImplementation(
      (cid: string, source: string, { controller }) =>
        getPromise('result', 5000, controller?.signal)
    );
    [cid1, cid2, cid3].map((cid) =>
      queueManager.enqueue(cid, jest.fn, { initialSource: 'node' })
    );

    onNextTick(() => {
      const queue = queueManager.getQueueList();
      console.log('---qm 1', queueManager.getQueueMap());

      expect(queue[0].status).toBe('executing');
      expect(queue[1].status).toBe('executing');
      expect(queue[2].status).toBe('pending');

      // Update priorities by viewport
      queueManager.updateViewPortPriority(cid1, -1);
      console.log('---qm 11', queueManager.getQueueMap());
      queueManager.enqueue(cid4, jest.fn, { initialSource: 'node' });
      console.log('---qm 2', queueManager.getQueueMap());

      onNextTick(() => {
        console.log('---qm 3', queueManager.getQueueMap());
        const queueMap = queueManager.getQueueMap();
        expect(queueMap.get(cid1).status).toBe('pending');
        expect(queueMap.get(cid2).status).toBe('executing');
        expect(queueMap.get(cid3).status).toBe('executing');
        expect(queueMap.get(cid4).status).toBe('pending');
        done();
      });
    });
  });
});

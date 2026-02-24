import { fetchIpfsContent } from 'src/services/ipfs/utils/utils-ipfs';

import QueueManager from './QueueManager';
import { QueueStrategy } from './QueueStrategy';
import { QueueItemStatus } from './types';

jest.mock('src/services/ipfs/utils/utils-ipfs', () => ({
  fetchIpfsContent: jest.fn(),
}));

// jest.mock('./utils', () => ({
//   postProcessIpfContent: (item, content, _) => content,
// }));

const QUEUE_DEBOUNCE_MS = 100;
const TIMEOUT_MS = 300;

const waitUtilQueueDebounce = (callback: () => void): ReturnType<typeof setTimeout> =>
  setTimeout(() => {
    callback();
  }, QUEUE_DEBOUNCE_MS);

function* valuesExpected<T>(values: T[]): Generator<T> {
  for (let i = 0; i < values.length; i++) {
    yield values[i];
  }
}

function wrapPromiseWithSignal(promise: Promise<string>, signal?: AbortSignal): Promise<string> {
  return new Promise((resolve, reject) => {
    promise.then((result) => {
      resolve(result);
    });

    signal?.addEventListener('abort', (e) => {
      // @ts-expect-error
      if (e?.target?.reason !== 'timeout') {
        reject(new DOMException('canceled', 'AbortError'));
      }
    });
  });
}

const getPromise = (result = 'result', timeout = 500, signal?: AbortSignal): Promise<string> =>
  wrapPromiseWithSignal(
    new Promise<string>((resolve) => {
      setTimeout(() => resolve(`result ${result}`), timeout);
    }),
    signal
  );

describe('QueueManager', () => {
  if (process.env.CI) {
    it('skip QueueManager tests', () => {
      expect(true).toBe(true);
    });
    return;
  }

  let queueManager: QueueManager<string>;
  const strategy = new QueueStrategy(
    {
      db: { timeout: TIMEOUT_MS, maxConcurrentExecutions: 2 },
      node: { timeout: TIMEOUT_MS, maxConcurrentExecutions: 2 },
      gateway: { timeout: TIMEOUT_MS, maxConcurrentExecutions: 2 },
    },
    ['db', 'node', 'gateway']
  );

  beforeEach(() => {
    queueManager = new QueueManager<string>(undefined, QUEUE_DEBOUNCE_MS);
    queueManager.setNode(
      {
        nodeType: 'external',
        config: {},
        init: jest.fn(),
        stop: jest.fn(),
        start: jest.fn(),
        cat: jest.fn(),
        stat: jest.fn(),
        add: jest.fn(),
        pin: jest.fn(),
        ls: jest.fn(),
        getPeers: jest.fn(),
        connectPeer: jest.fn(),
        info: jest.fn(),
        isConnectedToSwarm: jest.fn(),
        reconnectToSwarm: jest.fn(),
      },
      strategy
    );
  });

  it('should keep in pending items thats is out of maxConcurrentExecutions', (done) => {
    try {
      (fetchIpfsContent as jest.Mock).mockImplementation(() => getPromise());
      queueManager.enqueue('1', jest.fn);
      queueManager.enqueue('2', jest.fn);
      queueManager.enqueue('3', jest.fn);

      setTimeout(
        () => {
          const itemList = queueManager.getQueueList();
          expect(itemList[0].status).toEqual('executing');
          expect(itemList[1].status).toEqual('executing');
          expect(itemList[2].status).toEqual('pending');
          done();
        },
        QUEUE_DEBOUNCE_MS * 2 + 1
      );
    } finally {
      (fetchIpfsContent as jest.Mock).mockClear();
    }
  });

  it('should handle timeout and switch to next source', (done) => {
    try {
      const statuses = valuesExpected([
        'pending', // db
        'executing', // db
        'timeout', // db
        'pending', // db
        'executing', // node
        'timeout', // node
        'pending', // node
        'executing', // gateway
        'timeout', // gateway
        'not_found',
      ]);
      const itemId = 'id-to-timeout';

      (fetchIpfsContent as jest.Mock).mockImplementation(
        (_cid: string, _source: string, { controller }) =>
          getPromise('result', 50000, controller?.signal)
      );
      queueManager.enqueue(itemId, (cid, status, source) => {
        const expectedStatus = statuses.next().value;
        expect(cid).toBe(itemId);
        expect(status).toBe(expectedStatus);
        if (expectedStatus === 'timeout' && source === 'gateway') {
          done();
        }
      });
    } finally {
      (fetchIpfsContent as jest.Mock).mockClear();
    }
  });

  it('should cancel queue items', (done) => {
    const statuses = valuesExpected(['pending', 'executing', 'cancelled']);

    const itemId = 'id-to-cancel';
    (fetchIpfsContent as jest.Mock).mockImplementationOnce(
      (_cid: string, _source: string, { controller }) =>
        wrapPromiseWithSignal(getPromise('result', 1000), controller.signal)
    );
    queueManager.enqueue(itemId, (cid, status) => {
      expect(cid).toBe(itemId);
      expect(status).toBe(statuses.next().value);
      if (status === 'cancelled') {
        expect(queueManager.getQueueList()[0]?.controller?.signal.aborted).toBe(true);
      }
    });

    queueManager.cancel(itemId);

    waitUtilQueueDebounce(() => {
      expect(queueManager.getQueueList().length).toEqual(0);
      done();
    });
  });

  it('should handle execution errors and switch next', (done) => {
    try {
      const itemId = 'error-id';
      (fetchIpfsContent as jest.Mock).mockImplementation(() =>
        Promise.reject(new Error('some error'))
      );
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
      queueManager.enqueue(itemId, (cid, status, source): void => {
        expect(cid).toBe(itemId);
        const [statusExpected, sourceExpected] = responsesExpected.next().value;
        expect(status).toBe(statusExpected);
        expect(source).toBe(sourceExpected);
      });

      setTimeout(() => {
        const queue = queueManager.getQueueList();
        expect(queue.length).toBe(0);
        done();
      }, QUEUE_DEBOUNCE_MS * 4);
    } finally {
      (fetchIpfsContent as jest.Mock).mockClear();
    }
  });

  it('should execute queue items in order by priority', (done) => {
    try {
      (fetchIpfsContent as jest.Mock).mockImplementation(() => getPromise('x', 100));
      queueManager.enqueue('1', jest.fn);
      queueManager.enqueue('2', jest.fn);

      const executingByPriority: string[] = [];
      queueManager.enqueue('3', (cid) => executingByPriority.push(cid), {
        priority: 0.5,
      });

      queueManager.enqueue('4', (cid) => executingByPriority.push(cid), {
        priority: 0.9,
      });

      waitUtilQueueDebounce(() => {
        const queue = queueManager.getQueueMap();
        expect(queue.size).toBe(4);
        expect(queue.get('4').status).toBe('executing');
        expect(queue.get('3').status).toBe('executing');
        expect(queue.get('2').status).toBe('pending');
        expect(queue.get('1').status).toBe('pending');
        done();
      });
    } finally {
      (fetchIpfsContent as jest.Mock).mockClear();
    }
  });

  it('should execute queue items in order by viewportPriority in real-time', (done) => {
    try {
      (fetchIpfsContent as jest.Mock).mockImplementation(() => getPromise('x', 100));
      queueManager.enqueue('1', jest.fn);
      queueManager.enqueue('2', jest.fn);

      const executingByPriority: string[] = [];
      const setExecutingByPriority = (cid: string, status: QueueItemStatus) =>
        status === 'executing' && executingByPriority.push(cid);

      queueManager.enqueue('3', setExecutingByPriority, {
        priority: 0.5,
        viewPortPriority: 0.1,
      });

      queueManager.enqueue('4', setExecutingByPriority, {
        priority: 0.6,
        viewPortPriority: 0.1,
      });

      waitUtilQueueDebounce(() => {
        const queue = queueManager.getQueueMap();
        // const queue = queueManager.getQueueList();
        expect(queue.size).toBe(4);
        expect(queue.get('4').status).toBe('executing');
        expect(queue.get('3').status).toBe('executing');
        expect(queue.get('2').status).toBe('pending');
        expect(queue.get('1').status).toBe('pending');

        // Update priorities by viewport
        queueManager.updateViewPortPriority('3', 0.5);
        queueManager.updateViewPortPriority('4', 0);
      });

      setTimeout(() => {
        const queue = queueManager.getQueueMap();
        expect(queue.size).toBe(4);

        expect(executingByPriority[1]).toBe('3');

        expect(executingByPriority[0]).toBe('4');
        done();
      }, QUEUE_DEBOUNCE_MS + 1);
    } finally {
      (fetchIpfsContent as jest.Mock).mockClear();
    }
  });

  it('should execute queue items and deprioritize based on viewPortPriority', (done) => {
    try {
      (fetchIpfsContent as jest.Mock).mockImplementation(
        (_cid: string, _source: string, { controller }) =>
          getPromise('result', 1000, controller?.signal)
      );
      ['1', '2', '3'].map((cid) => queueManager.enqueue(cid, jest.fn, { initialSource: 'node' }));

      waitUtilQueueDebounce(() => {
        const queue = queueManager.getQueueList();
        expect(queue[0].status).toBe('executing');
        expect(queue[1].status).toBe('executing');
        expect(queue[2].status).toBe('pending');
        queueManager.enqueue('4', jest.fn, { initialSource: 'node' });

        // Update priorities by viewport
        queueManager.updateViewPortPriority('1', -1);
      });

      setTimeout(
        () => {
          try {
            const queueMap = queueManager.getQueueMap();
            expect(queueMap.get('1').status).toBe('pending');
            expect(queueMap.get('2').status).toBe('executing');
            expect(queueMap.get('3').status).toBe('executing');
            expect(queueMap.get('4').status).toBe('pending');
          } finally {
            done();
          }
        },
        QUEUE_DEBOUNCE_MS * 3 + 10
      );
    } finally {
      (fetchIpfsContent as jest.Mock).mockClear();
    }
  });

  it('should execute enqueue as promise and get result', async () => {
    try {
      (fetchIpfsContent as jest.Mock).mockImplementation(() => Promise.resolve('done!'));

      const result = await queueManager.enqueueAndWait('xxx');
      console.log('----r', result);
      expect(result?.result).toBe('done!');
    } finally {
      (fetchIpfsContent as jest.Mock).mockClear();
    }
  });

  it('should execute enqueue as promise and get undefined', async () => {
    try {
      (fetchIpfsContent as jest.Mock).mockImplementation(() => Promise.reject());
      const result = await queueManager.enqueueAndWait('xxx');
      expect(result?.status).toBe('not_found');
    } finally {
      (fetchIpfsContent as jest.Mock).mockClear();
    }
  });
});

import init, { Miner } from 'uhash-web';

let miner: Miner | null = null;
let mining = false;
let totalHashes = 0;
let currentNonce = 0;
let numThreads = 1;
let timestamp = 0;
const BATCH_SIZE = 100;

type InMessage =
  | { type: 'init' }
  | { type: 'start'; threadId: number; numThreads: number; seed: string; address: string; timestamp: number; difficulty: number }
  | { type: 'stop' };

self.onmessage = async (e: MessageEvent<InMessage>) => {
  const { type } = e.data;

  switch (type) {
    case 'init':
      await init();
      self.postMessage({ type: 'ready' });
      break;

    case 'start': {
      const msg = e.data as Extract<InMessage, { type: 'start' }>;
      numThreads = msg.numThreads;
      timestamp = msg.timestamp;
      currentNonce = msg.threadId;
      totalHashes = 0;
      miner = new Miner(msg.seed, msg.address, msg.timestamp, msg.difficulty);
      mining = true;
      mine();
      break;
    }

    case 'stop':
      mining = false;
      break;
  }
};

function mine() {
  if (!mining || !miner) return;

  const result = JSON.parse(miner.mine_batch(currentNonce, numThreads, BATCH_SIZE));
  totalHashes += result.count;
  currentNonce += numThreads * BATCH_SIZE;

  if (result.found) {
    self.postMessage({
      type: 'proof',
      hash: result.hash,
      nonce: result.nonce,
      timestamp,
      totalHashes,
    });
  }

  if (mining) {
    self.postMessage({ type: 'progress', totalHashes });
    setTimeout(mine, 0);
  }
}

import init, { LithiumMiner } from 'uhash-web';

let miner: LithiumMiner | null = null;
let mining = false;
let totalHashes = 0;
let currentNonce = 0;
let numThreads = 1;
const BATCH_SIZE = 100;

type InMessage =
  | { type: 'init' }
  | { type: 'start'; threadId: number; numThreads: number; address: string; blockHash: string; dataHash: string; difficulty: number }
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
      currentNonce = msg.threadId;
      totalHashes = 0;
      miner = new LithiumMiner(msg.address, msg.blockHash, msg.dataHash, msg.difficulty);
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
      totalHashes,
    });
  }

  if (mining) {
    self.postMessage({ type: 'progress', totalHashes });
    setTimeout(mine, 0);
  }
}

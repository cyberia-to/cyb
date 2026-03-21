import init, { ChallengeMiner } from 'uhash-web';

let miner: ChallengeMiner | null = null;
let mining = false;
let totalHashes = 0;
let currentNonce = 0;
let numThreads = 1;
const BATCH_SIZE = 100;

// Throttle progress messages: max 2 per second
const PROGRESS_INTERVAL_MS = 500;
let lastProgressTime = 0;

type InMessage =
  | { type: 'init' }
  | { type: 'start'; threadId: number; numThreads: number; challenge: string; difficulty: number }
  | { type: 'stop' };

let currentChallenge = '';

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
      lastProgressTime = 0;
      currentChallenge = msg.challenge;
      miner = new ChallengeMiner(msg.challenge, msg.difficulty);
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

  // Run multiple batches before yielding to keep CPU busy but allow message processing
  const batchesPerYield = 10;
  for (let b = 0; b < batchesPerYield; b++) {
    if (!mining || !miner) return;

    const result = JSON.parse(miner.mine_batch(currentNonce, numThreads, BATCH_SIZE));
    totalHashes += result.count;
    currentNonce += numThreads * BATCH_SIZE;

    if (result.found) {
      self.postMessage({
        type: 'proof',
        hash: result.hash,
        nonce: result.nonce,
        challenge: currentChallenge,
        totalHashes,
      });
    }
  }

  if (mining) {
    // Throttle progress messages to avoid flooding the main thread
    const now = performance.now();
    if (now - lastProgressTime >= PROGRESS_INTERVAL_MS) {
      self.postMessage({ type: 'progress', totalHashes });
      lastProgressTime = now;
    }

    // Use setTimeout to yield to the event loop so 'stop' messages can be processed
    setTimeout(mine, 1);
  }
}

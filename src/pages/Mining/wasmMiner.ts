export interface WasmMiningStatus {
  mining: boolean;
  total_hashes: number;
  elapsed_secs: number;
  hashrate: number;
  pending_proofs: number;
}

type WorkerMessage =
  | { type: 'ready' }
  | { type: 'progress'; totalHashes: number }
  | { type: 'proof'; hash: string; nonce: number; challenge: string; totalHashes: number };

export type FoundProof = { hash: string; nonce: number; challenge?: string };

type HashSnapshot = { time: number; hashes: number };

const ROLLING_WINDOW_MS = 30_000;

export class WasmMiner {
  private workers: Worker[] = [];
  private workerLastReported: number[] = [];
  private mining = false;
  private startTime = 0;
  private pendingProofs: FoundProof[] = [];
  private numThreads: number;
  private hashSnapshots: HashSnapshot[] = [];
  private currentChallenge = '';
  private totalHashes = 0;

  constructor(numThreads: number) {
    this.numThreads = numThreads;
  }

  async init(): Promise<void> {
    this.workers = [];
    this.workerLastReported = [];

    const readyPromises: Promise<void>[] = [];

    for (let i = 0; i < this.numThreads; i++) {
      const worker = new Worker(new URL('./miningWorker.ts', import.meta.url));
      this.workers.push(worker);
      this.workerLastReported.push(0);

      const readyPromise = new Promise<void>((resolve) => {
        const handler = (e: MessageEvent<WorkerMessage>) => {
          if (e.data.type === 'ready') {
            worker.removeEventListener('message', handler);
            resolve();
          }
        };
        worker.addEventListener('message', handler);
      });
      readyPromises.push(readyPromise);

      worker.postMessage({ type: 'init' });
    }

    await Promise.all(readyPromises);
    this.setupMessageHandlers();
  }

  private setupMessageHandlers(): void {
    this.workers.forEach((worker, index) => {
      worker.onmessage = (e: MessageEvent<WorkerMessage>) => {
        const msg = e.data;

        if (msg.type === 'progress' || msg.type === 'proof') {
          // Incremental counting: only add positive deltas.
          // When a worker restarts (totalHashes drops to 0), delta < 0 → skip.
          // Next message after restart has a small positive delta → resumes counting.
          const delta = msg.totalHashes - this.workerLastReported[index];
          if (delta > 0) {
            this.totalHashes += delta;
          }
          this.workerLastReported[index] = msg.totalHashes;

          if (msg.type === 'proof') {
            this.pendingProofs.push({
              hash: msg.hash,
              nonce: msg.nonce,
              challenge: msg.challenge,
            });
          }
        }
      };
    });
  }

  start(challenge: string, difficulty: number): void {
    if (!this.mining) {
      // Fresh start — reset everything
      this.startTime = Date.now();
      this.hashSnapshots = [];
      this.totalHashes = 0;
      this.workerLastReported = this.workers.map(() => 0);
      this.pendingProofs = [];
    }
    // Challenge change while mining: just send new start to workers.
    // workerLastReported stays — delta tracking handles the restart automatically.
    this.mining = true;
    this.currentChallenge = challenge;

    this.workers.forEach((worker, i) => {
      worker.postMessage({
        type: 'start',
        threadId: i,
        numThreads: this.numThreads,
        challenge,
        difficulty,
      });
    });
  }

  stop(): void {
    this.stopWorkers();
  }

  private stopWorkers(): void {
    this.mining = false;
    this.workers.forEach((worker) => {
      worker.postMessage({ type: 'stop' });
    });
  }

  getStatus(): WasmMiningStatus {
    const now = Date.now();
    const elapsedSecs = this.mining || this.totalHashes > 0
      ? (now - this.startTime) / 1000
      : 0;

    // Update rolling window snapshots (called every ~500ms from poll)
    if (this.mining) {
      this.hashSnapshots.push({ time: now, hashes: this.totalHashes });
      const cutoff = now - ROLLING_WINDOW_MS;
      this.hashSnapshots = this.hashSnapshots.filter(
        (s) => s.time >= cutoff
      );
    }

    // 30s rolling hashrate from snapshots
    let hashrate = 0;
    if (this.hashSnapshots.length >= 2) {
      const oldest = this.hashSnapshots[0];
      const newest = this.hashSnapshots[this.hashSnapshots.length - 1];
      const dt = (newest.time - oldest.time) / 1000;
      if (dt > 0.5) {
        hashrate = (newest.hashes - oldest.hashes) / dt;
      }
    }

    // Fallback to lifetime average during warmup (<1s of data)
    if (hashrate === 0 && elapsedSecs > 0) {
      hashrate = this.totalHashes / elapsedSecs;
    }

    return {
      mining: this.mining,
      total_hashes: this.totalHashes,
      elapsed_secs: elapsedSecs,
      hashrate,
      pending_proofs: this.pendingProofs.length,
    };
  }

  takeProofs(): FoundProof[] {
    const proofs = this.pendingProofs;
    this.pendingProofs = [];
    return proofs;
  }

  destroy(): void {
    console.log('[WasmMiner] destroy: terminating', this.workers.length, 'workers');
    this.mining = false;
    this.workers.forEach((worker) => worker.terminate());
    this.workers = [];
    this.workerLastReported = [];
    this.totalHashes = 0;
    this.hashSnapshots = [];
  }
}

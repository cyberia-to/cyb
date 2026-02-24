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
  | { type: 'proof'; hash: string; nonce: number; totalHashes: number };

export type FoundProof = { hash: string; nonce: number };

export class WasmMiner {
  private workers: Worker[] = [];
  private workerHashes: number[] = [];
  private mining = false;
  private startTime = 0;
  private pendingProofs: FoundProof[] = [];
  private numThreads: number;

  constructor(numThreads: number) {
    this.numThreads = numThreads;
  }

  async init(): Promise<void> {
    this.workers = [];
    this.workerHashes = [];

    const readyPromises: Promise<void>[] = [];

    for (let i = 0; i < this.numThreads; i++) {
      const worker = new Worker(new URL('./miningWorker.ts', import.meta.url));
      this.workers.push(worker);
      this.workerHashes.push(0);

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

        if (msg.type === 'progress') {
          this.workerHashes[index] = msg.totalHashes;
        } else if (msg.type === 'proof') {
          this.workerHashes[index] = msg.totalHashes;
          this.pendingProofs.push({
            hash: msg.hash,
            nonce: msg.nonce,
          });
        }
      };
    });
  }

  start(address: string, blockHash: string, dataHash: string, difficulty: number): void {
    this.pendingProofs = [];
    this.workerHashes = this.workers.map(() => 0);
    this.startTime = Date.now();
    this.mining = true;

    this.workers.forEach((worker, i) => {
      worker.postMessage({
        type: 'start',
        threadId: i,
        numThreads: this.numThreads,
        address,
        blockHash,
        dataHash,
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
    const totalHashes = this.workerHashes.reduce((a, b) => a + b, 0);
    const elapsedSecs = this.mining || totalHashes > 0
      ? (Date.now() - this.startTime) / 1000
      : 0;
    const hashrate = elapsedSecs > 0 ? totalHashes / elapsedSecs : 0;

    return {
      mining: this.mining,
      total_hashes: totalHashes,
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
    this.mining = false;
    this.workers.forEach((worker) => worker.terminate());
    this.workers = [];
    this.workerHashes = [];
  }
}

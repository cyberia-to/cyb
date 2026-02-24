import { ProgressTracking } from 'src/services/backend/types/services';

const ROLLING_WINDOW = 10;

type onProgressUpdateFunc = (progress: ProgressTracking) => void;

type RequestRecord = {
  timestamp: number;
  itemCount: number;
};

// eslint-disable-next-line import/no-unused-modules, import/prefer-default-export
export class ProgressTracker {
  private requestRecords: RequestRecord[] = [];

  private totalRequests = 0;

  private completedRequests = 0;

  private estimatedTime = -1;

  private onProgressUpdate?: onProgressUpdateFunc;

  public get progress(): ProgressTracking {
    return {
      totalCount: this.totalRequests,
      completeCount: this.completedRequests,
      estimatedTime: this.estimatedTime,
    };
  }

  constructor(onProgressUpdate?: onProgressUpdateFunc) {
    this.onProgressUpdate = onProgressUpdate;
  }

  public start(totalRequests: number, batchSize = 1) {
    this.totalRequests = totalRequests;
    this.requestRecords = [];
    this.completedRequests = 0;
    this.estimatedTime = -1;
    this.batchSize = batchSize;

    return this.progress;
  }

  public add(extraRequests: number) {
    this.totalRequests += extraRequests;

    return this.progress;
  }

  public trackProgress(processedCount: number) {
    this.addRequestRecord(processedCount);

    if (this.requestRecords.length > ROLLING_WINDOW) {
      this.requestRecords.shift();
    }

    if (this.requestRecords.length > 1) {
      const averageTimePerItem = this.calculateAverageTimePerItem();
      const remainingRequests = this.totalRequests - this.completedRequests;
      const estimatedRemainingItems = remainingRequests * processedCount; // Assuming remaining requests will process the same number of items
      const estimatedRemainingTime = averageTimePerItem * estimatedRemainingItems;

      this.completedRequests += processedCount;
      this.estimatedTime = Math.round(estimatedRemainingTime); // Convert to seconds;
      this.onProgressUpdate?.(this.progress);
    }

    return this.progress;
  }

  private addRequestRecord(itemCount: number) {
    this.requestRecords.push({ timestamp: Date.now(), itemCount });
  }

  private calculateAverageTimePerItem(): number {
    let totalDiff = 0;
    let totalItems = 0;

    for (let i = 1; i < this.requestRecords.length; i++) {
      const timeDiff = this.requestRecords[i].timestamp - this.requestRecords[i - 1].timestamp;
      const { itemCount } = this.requestRecords[i];

      totalDiff += timeDiff * itemCount;
      totalItems += itemCount;
    }

    return totalItems === 0 ? 0 : totalDiff / totalItems;
  }
}

import { IQueueStrategy, QueueSettings, QueueSource } from './types';

export class QueueStrategy implements IQueueStrategy {
  settings: QueueSettings;

  order: QueueSource[];

  constructor(settings: QueueSettings, order: QueueSource[]) {
    this.settings = settings;
    this.order = order;
  }

  getNextSource(source: QueueSource): QueueSource | undefined {
    const index = this.order.indexOf(source);
    return index < this.order.length ? this.order[index + 1] : undefined;
  }
}

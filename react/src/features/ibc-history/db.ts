import Dexie, { Table } from 'dexie';
import { HistoriesItem } from './HistoriesItem';

export class IbcHistories extends Dexie {
  historiesItems!: Table<HistoriesItem>;

  constructor() {
    super('ibc-histories');
    this.version(1).stores({
      historiesItems: '++id, address, txHash',
    });
  }

  deleteList(address: string) {
    return this.transaction('rw', this.historiesItems, () => {
      this.historiesItems.where({ address }).delete();
    });
  }
}

export const db = new IbcHistories();

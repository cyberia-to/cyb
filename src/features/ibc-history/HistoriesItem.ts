import { Coin } from '@cosmjs/launchpad';

export enum StatusTx {
  PENDING = 'pending',
  TIMEOUT = 'timeout',
  COMPLETE = 'complete',
  REFUNDED = 'refunded',
}

export interface HistoriesItem {
  id?: number;
  address: string;
  txHash: string;
  sourceChainId: string;
  sourceChannelId: string;
  destChainId: string;
  destChannelId: string;
  sequence: string;
  sender: string;
  recipient: string;
  amount: Coin;
  timeoutHeight?: string;
  timeoutTimestamp?: string;
  createdAt: number | string;
  status: StatusTx;
}

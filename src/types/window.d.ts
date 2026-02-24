import { Window as KeplrWindow } from '@keplr-wallet/types';

interface EthereumProvider {
  request: (args: { method: string; params?: unknown[] }) => Promise<unknown>;
  on?: (event: string, handler: (...args: unknown[]) => void) => void;
  removeListener?: (event: string, handler: (...args: unknown[]) => void) => void;
  isMetaMask?: boolean;
}

declare global {
  interface Window extends KeplrWindow {
    ethereum?: EthereumProvider;

    // for our window things
    cyb: any;
  }
}

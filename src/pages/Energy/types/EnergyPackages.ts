import { Coin } from '@cosmjs/stargate';
import { SwapTokensWithRoutes } from './swap';

// eslint-disable-next-line import/no-unused-modules, import/prefer-default-export
export type EnergyPackagesKey = '10' | '100' | '1000';

export type EnergyPackageSwapRoutes = {
  keyPackage: string;
  tokenIn: Coin;
  tokenOut: Coin[];
  swapInfo: SwapTokensWithRoutes[];
};

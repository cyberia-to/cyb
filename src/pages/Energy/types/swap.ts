import { Coin } from '@cosmjs/stargate';
import { SwapAmountInRoute } from 'osmojs/osmosis/poolmanager/v1beta1/swap_route';
import { Token } from './token';

export type Swap = {
  to: Token;
  from: Token;
};

export type SwapTokensWithRoutes = {
  swap: {
    tokenIn: Coin;
    tokenOut: Coin;
  };
  routes: SwapAmountInRoute[];
  tokenOutConvert: Coin;
};

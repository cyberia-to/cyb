import { Osmosis } from './assets';

export const symbol = ['hydrogen', 'ampere', 'volt'];
// eslint-disable-next-line import/prefer-default-export
export const assetsBuy = Array.from(
  new Map(
    Osmosis.Assets.filter((item) => symbol.includes(item.display)).map(
      (item) => [item.base, item]
    )
  ).values()
);

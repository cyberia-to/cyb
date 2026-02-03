/* eslint-disable camelcase */
import { chains } from 'chain-registry';
import { Asset } from '@chain-registry/types';
import { CoinDenom } from '@chain-registry/utils';
import { assets as assetsList } from 'chain-registry';
import { assets, asset_list } from '@chain-registry/osmosis';

const defaultChainName = 'osmosis';

type CoinGeckoId = string;

type CoinDenomToAsset = Record<CoinDenom, Asset>;
type CoinGeckoIdToAsset = Record<CoinGeckoId, Asset>;
type CoinGeckoIds = CoinGeckoId[];

export const osmosisAssetsList = assetsList.filter(
  (a) => a.chain_name === 'osmosis'
);

export const OsmosisAssets = [...assets.assets, ...asset_list.assets].filter(
  ({ type_asset }) => type_asset !== 'ics20'
) as Asset[];

// OsmosisAssets.WithCoinGeckoId = OsmosisAssets.filter(({ coingecko_id }) =>
//   Boolean(coingecko_id)
// );

// OsmosisAssets.CoinGeckoIds = OsmosisAssets.WithCoinGeckoId.map(
//   ({ coingecko_id }) => coingecko_id
// ) as CoinGeckoId[];

// OsmosisAssets.CoinGeckoIdToAsset = OsmosisAssets.WithCoinGeckoId.reduce(
//   (cache, asset) => ({ ...cache, [asset.coingecko_id!]: asset }),
//   {}
// );

export const CoinDenomToAsset = OsmosisAssets.reduce(
  (cache, asset) => ({ ...cache, [asset.base]: asset }),
  {}
) as CoinDenomToAsset;

function getChainByDenom(denom: CoinDenom) {
  let chainName = '';
  if (assets.assets.find(({ base }) => base === denom)) {
    chainName = defaultChainName;
  } else {
    const asset = asset_list.assets.find(({ base }) => base === denom);
    chainName = asset?.traces?.[0].counterparty.chain_name || '';
  }
  return chainName
    ? chains.find(({ chain_name }) => chain_name === chainName)
    : null;
}

export const Osmosis = {
  getChainByDenom,
  Assets: OsmosisAssets,
  CoinDenomToAsset,
  AssetsList: osmosisAssetsList,
};

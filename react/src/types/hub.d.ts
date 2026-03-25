import { ObjectKey } from './data';

export type Channel = {
  id: number;
  active: 'true' | 'false';
  source_chain_id: string;
  destination_chain_id: string;
  source_channel_id: string;
  destination_channel_id: string;
  explorer_url: string;
  particle: string;
};

export type Token = {
  id: number;
  ticker: string;
  chain_id: string;
  contract: string;
  decimals: string;
  channel_id: string;
  logo: string;
  particle: string;
};

export type Network = {
  id: number;
  name: string;
  chain_id: string;
  prefix: string;
  genesis_hash: string;
  unbonding_period: string;
  protocol: string;
  logo: string;
  particle: string;
};

export type TokenList = ObjectKey<Token>;

export type NetworkList = ObjectKey<Network>;

export type ChannelList = ObjectKey<Channel>;

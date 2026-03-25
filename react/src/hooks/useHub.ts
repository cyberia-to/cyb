import { CyberClient } from '@cybercongress/cyber-js';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { CHAIN_ID } from 'src/constants/config';
import { HUB_CHANNELS, HUB_NETWORKS, HUB_TOKENS } from 'src/constants/hubContracts';
import { useQueryClient } from 'src/contexts/queryClient';
import { setChannels, setNetworks, setTokens } from 'src/pages/Hub/redux/hub';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { Option } from 'src/types';
import { Channel, ChannelList, Network, NetworkList, Token, TokenList } from 'src/types/hub';

enum TypeFetcher {
  NETWORKS = HUB_NETWORKS,
  TOKENS = HUB_TOKENS,
  CHANNELS = HUB_CHANNELS,
}

const QUERY_MSG = {
  get_entries: {},
};

const fetcher = (client: Option<CyberClient>, type: TypeFetcher) => {
  if (!client) {
    return undefined;
  }

  return client.queryContractSmart(type, QUERY_MSG);
};

export function useNetworks() {
  const dispatch = useAppDispatch();
  const { networks: networksStorage } = useAppSelector((state) => state.hub);
  const queryClient = useQueryClient();
  const [networksData, setNetworksData] = useState<Option<NetworkList>>(networksStorage);
  const { data } = useQuery(['hub-networks'], () => fetcher(queryClient, TypeFetcher.NETWORKS), {
    enabled: Boolean(queryClient),
  });

  useEffect(() => {
    const objectMappedResult: NetworkList = {};
    if (data) {
      data.entries.forEach((row: Network) => {
        objectMappedResult[row.chain_id] = row;
      });
    }
    if (Object.keys(objectMappedResult).length > 0) {
      setNetworksData(objectMappedResult);
      dispatch(setNetworks(objectMappedResult));
    }
  }, [data, dispatch]);

  return { networks: networksData };
}

export function useTokens() {
  const dispatch = useAppDispatch();
  const { tokens: tokensStorage } = useAppSelector((state) => state.hub);
  const queryClient = useQueryClient();
  const [tokensData, setTokensData] = useState<Option<TokenList>>(tokensStorage);
  const { data } = useQuery(['hub-tokens'], () => fetcher(queryClient, TypeFetcher.TOKENS), {
    enabled: Boolean(queryClient),
  });

  useEffect(() => {
    const objectMappedResult: TokenList = {};
    if (data) {
      data.entries.forEach((row: Token) => {
        if (row.chain_id === CHAIN_ID) {
          const { contract } = row;
          const ticker =
            contract.indexOf('native') !== -1 ? contract.replace('native/', '') : contract;
          objectMappedResult[ticker] = row;
        }
      });

      if (Object.keys(objectMappedResult).length > 0) {
        setTokensData(objectMappedResult);
        dispatch(setTokens(objectMappedResult));
      }
    }
  }, [data, dispatch]);

  return { tokens: tokensData };
}

export function useChannels() {
  const dispatch = useAppDispatch();
  const { channels: channelsStorage } = useAppSelector((state) => state.hub);
  const queryClient = useQueryClient();
  const [channelsData, setChannelsData] = useState<Option<ChannelList>>(channelsStorage);
  const { data } = useQuery(['hub-channels'], () => fetcher(queryClient, TypeFetcher.CHANNELS), {
    enabled: Boolean(queryClient),
  });

  useEffect(() => {
    const objectMappedResult: ChannelList = {};
    if (data) {
      data.entries.forEach((row: Channel) => {
        if (row.active === 'true') {
          objectMappedResult[row.destination_chain_id] = row;
        }
      });

      if (Object.keys(objectMappedResult).length > 0) {
        setChannelsData(objectMappedResult);
        dispatch(setChannels(objectMappedResult));
      }
    }
  }, [data, dispatch]);

  return { channels: channelsData };
}

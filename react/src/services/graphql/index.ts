import { ApolloClient, ApolloLink, HttpLink, InMemoryCache, split } from '@apollo/client';
import { GraphQLWsLink } from '@apollo/client/link/subscriptions';
import { getMainDefinition } from '@apollo/client/utilities';
import { OperationDefinitionNode } from 'graphql';
import { createClient } from 'graphql-ws';
import { INDEX_HTTPS, INDEX_WEBSOCKET } from 'src/constants/config';

const httpLink = new HttpLink({
  uri: INDEX_HTTPS,
  headers: {
    'content-type': 'application/json',
    authorization: '',
  },
});

const wsLink = new GraphQLWsLink(
  createClient({
    url: INDEX_WEBSOCKET,
  })
);

const terminatingLink = split(
  ({ query }) => {
    const { kind, operation } = getMainDefinition(query) as OperationDefinitionNode;
    return kind === 'OperationDefinition' && operation === 'subscription';
  },
  wsLink,
  httpLink
);

const link = ApolloLink.from([terminatingLink]);

const cache = new InMemoryCache();

// TODO: replace with @apollo/client
const client = new ApolloClient({
  link,
  cache,
});

const apolloClient = client;

export default apolloClient;

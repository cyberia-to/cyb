import { CodegenConfig } from '@graphql-codegen/cli';

// import { INDEX_HTTPS } from './src/constants/config';
// require('dotenv').config();
// TODO: need to use config variables somehow

const schemaUrl = 'https://index.bostrom.cybernode.ai/v1/graphql'; // process.env.NEXT_PUBLIC_GRAPHQL_HOST;

const config: CodegenConfig = {
  overwrite: true,
  schema: schemaUrl,
  debug: true,
  documents: ['src/**/*.graphql'],
  config: {
    withHooks: true,
    skipTypename: true,
  },
  generates: {
    'src/generated/graphql.ts': {
      plugins: [
        'typescript',
        'typescript-operations',
        'typescript-react-apollo',
      ],
    },
  },
};

export default config;

const { merge } = require('webpack-merge');
// TODO: workbox-webpack-plugin uses webpack.EntryPlugin internally,
// which is incompatible with Rspack. Replace with inject-manifest-plugin
// or @nicman23/rspack-workbox-plugin when available.
// const WorkboxPlugin = require('workbox-webpack-plugin');
const rspack = require('@rspack/core');
const commonConfig = require('./rspack.config.common');

module.exports = merge(commonConfig, {
  mode: 'production',
  devtool: 'source-map',
  output: {
    filename: '[name].[contenthash:8].js',
    chunkFilename: '[name].[contenthash:8].chunk.js',
  },
  optimization: {
    nodeEnv: 'production',
    concatenateModules: true,
    removeAvailableModules: true,
    splitChunks: {
      chunks: 'all',
      minSize: 2024000,
      maxSize: 3024000,
    },
    minimize: true,
  },
  plugins: [
    ...(process.env.NETLIFY
      ? [
          new rspack.NormalModuleReplacementPlugin(
            /react-force-graph/,
            (resource) => {
              resource.request = 'src/../netlify/mocks/ReactForceGraph';
            }
          ),
        ]
      : []),
    // WorkboxPlugin disabled — see TODO above
    // new WorkboxPlugin.InjectManifest({
    //   swSrc: 'src/services/service-worker/service-worker.ts',
    //   swDest: 'service-worker.js',
    //   maximumFileSizeToCacheInBytes: 50 * 1024 * 1024,
    // }),
  ],
});

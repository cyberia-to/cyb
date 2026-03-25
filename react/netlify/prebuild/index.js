// eslint-disable-next-line import/no-unused-modules
module.exports.onPreBuild = function ({ netlifyConfig }) {
  netlifyConfig.build.environment.COMMIT_SHA = process.env.COMMIT_REF;
  netlifyConfig.build.environment.BRANCH = process.env.BRANCH;
};

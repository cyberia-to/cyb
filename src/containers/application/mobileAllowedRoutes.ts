import { routes } from 'src/routes';

// eslint-disable-next-line import/prefer-default-export
export const mobileAllowedRoutes = [
  // `/`
  routes.home.path,

  // oracle
  routes.oracle,
  routes.oracle.ask,
  routes.oracle.learn,
  routes.oracle.routes.blocks.path,
  routes.oracle.routes.txs.path,
  routes.oracle.routes.stats.path,
  routes.oracle.routes.particles.path,

  routes.robot.path,

  routes.warp.path,
  routes.social.path,

  routes.brain,

  routes.mining.path,

  routes.settings.path,
  routes.keys.path,

  routes.teleport.path,
  routes.sphere.path,
  routes.portal.path,
  routes.citizenship.path,

  'network/bostrom/tx/*',
];

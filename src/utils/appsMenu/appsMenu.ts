import portal from 'images/space-pussy.svg';
import portalGlow from 'src/image/space-pussy-glow.svg';
import { CHAIN_ID } from 'src/constants/config';
import { cybernetRoutes } from 'src/features/cybernet/ui/routes';
import congress from 'src/image/new_icons/congress.svg';
import hfr from 'src/image/new_icons/hfr.svg';
import mining from 'src/image/new_icons/mining.svg';
import nebulaIcon from 'src/image/new_icons/nebula.svg';
import oracle from 'src/image/new_icons/oracle.svg';
import robot from 'src/image/new_icons/robot.svg';
import senate from 'src/image/new_icons/senate.svg';
import shpere from 'src/image/new_icons/sphere.svg';
import teleport from 'src/image/new_icons/teleport.svg';
import warp from 'src/image/new_icons/warp.svg';
import { routes } from 'src/routes';
import { Networks } from 'src/types/networks';
import { isPussyChain } from '../chains/pussy';

const getMenuItems = () => {
  const listItemMenu = [
    {
      name: 'robot',
      icon: robot,
      to: '/robot',
      subItems: [
        { name: 'sense', to: 'sense', icon: require('./images/dna.png') },
        { name: 'brain', to: 'brain', icon: require('./images/brain.png') },
        {
          name: 'time',
          to: 'time',
          icon: require('./images/horizontal-traffic-light.png'),
        },
        { name: 'sigma', to: 'sigma', icon: require('./images/sigma@2x.png') },
      ],
      // subItems: myRobotLinks,
    },
    {
      name: 'Oracle',
      to: '/',
      icon: oracle,
      subItems: [
        {
          name: 'Particles',
          to: '/particles',
          icon: require('./images/tag@2x.png'),
        },
        {
          name: 'brain',
          to: routes.brain.path,
          icon: '🧠',
        },
        {
          name: 'Stats',
          to: '/oracle/stats',
          icon: require('./images/avatar@2x.png'),
        },
        {
          name: 'Blocks',
          to: '/network/bostrom/blocks',
          icon: require('./images/gold-blocks.png'),
        },
        {
          name: 'Txs',
          to: '/network/bostrom/tx',
          icon: require('./images/horizontal-traffic-light.png'),
        },
        {
          name: 'Contracts',
          to: '/contracts',
          icon: require('./images/doc@2x.png'),
        },
        { name: 'Libs', to: '/libs', icon: require('./images/database.png') },
      ],
    },
    {
      name: 'Docs',
      to: 'https://docs.cyb.ai',
      subItems: [],
      icon: require('src/image/new_icons/docs.svg'),
    },
    { name: 'Nebula', to: '/nebula', subItems: [], icon: nebulaIcon },
    {
      name: 'Teleport',
      to: '/teleport',
      icon: teleport,
      active: false,
      subItems: [
        {
          name: 'Send',
          to: routes.teleport.send.path,
          icon: require('./images/rocket-send@2x.png'),
        },
        {
          name: 'Bridge',
          to: routes.teleport.bridge.path,
          icon: require('./images/arrow-swap@2x.png'),
        },
        {
          name: 'Swap',
          to: routes.teleport.swap.path,
          icon: require('./images/swap.png'),
        },
      ],
    },
    {
      name: 'Warp',
      icon: warp,
      to: '/warp',
      subItems: [
        {
          name: 'Add liquidity',
          to: '/warp/add-liquidity',
          icon: require('images/msgs_ic_pooladd.svg'),
        },
        {
          name: 'Create pool',
          to: '/warp/create-pool',
          icon: require('images/flask-outline.svg'),
        },
        {
          name: 'Sub liquidity',
          to: '/warp/sub-liquidity',
          icon: require('images/msgs_ic_poolremove.svg'),
        },
      ],
    },
    {
      name: 'Sphere',
      icon: shpere,
      to: routes.sphere.path,
      subItems: [],
    },
    { name: 'HFR', icon: hfr, to: '/hfr', subItems: [] },
    { name: 'Senate', icon: senate, to: '/senate', subItems: [] },

    !isPussyChain
      ? {
          name: 'Cyberver 🟣',
          icon: require('src/image/new_icons/cyberver.svg'),
          to: 'https://spacepussy.ai/cyberver',
          subItems: [],
        }
      : {
          name: 'cyberver',
          icon: require('./images/cyberver.png'),
          to: '/cyberver',
          subItems: [
            {
              name: '👑  board',
              to: '/cyberver/faculties/board',
              matchPathname: cybernetRoutes.subnet.path.replace(':nameOrUid', 'board'),
            },
            {
              name: '🏫  faculties',
              to: '/cyberver/faculties',
              matchPathname: cybernetRoutes.subnets.path,
            },
            {
              name: '💼  mentors',
              to: '/cyberver/mentors',
              matchPathname: cybernetRoutes.delegators.path,
            },
            {
              name: '👨‍🎓  my mentor',
              to: '/cyberver/mentors/my',
              matchPathname: cybernetRoutes.myMentor.path,
            },
            {
              name: '👨‍🎓  my learner',
              to: '/cyberver/learners/my',
              matchPathname: cybernetRoutes.myLearner.path,
            },
            {
              name: '𝚺 sigma',
              to: '/cyberver/sigma',
            },
          ],
        },

    { name: 'About', icon: congress, to: routes.social.path, subItems: [] },
    {
      name: 'Studio',
      icon: require('./images/studio.svg'),
      to: routes.studio.path,
      subItems: [],
    },
  ];

  if (CHAIN_ID === Networks.BOSTROM) {
    listItemMenu.splice(2, 0, {
      name: 'Portal',
      icon: portalGlow,
      largeIcon: portal,
      to: '/portal',
      disabled: true,
      subItems: [
        {
          name: 'Citizenship',
          to: '/citizenship',
          icon: require('./images/identification-card.png'),
        },
        {
          name: 'Gift',
          to: '/gift',
          icon: require('./images/wrapped-gift.png'),
        },
        {
          name: 'Map',
          to: routes.portal.routes.map.path,
          icon: require('./images/world-map.png'),
        },
        {
          name: 'AOS',
          to: '/genesis',
          icon: require('./images/aos.png'),
        },
      ],
    });
  }
  return listItemMenu.filter((item) => item);
};

const MOBILE_APPS = new Set(['robot', 'Oracle', 'Teleport', 'Portal']);

export const getMobileMenuItems = () =>
  getMenuItems().filter((item) => MOBILE_APPS.has(item.name));

export default getMenuItems;

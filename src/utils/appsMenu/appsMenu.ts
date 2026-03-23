import portal from 'images/space-pussy.svg';
import portalGlow from 'src/image/space-pussy-glow.svg';
import { CHAIN_ID } from 'src/constants/config';
import oracle from 'src/image/new_icons/oracle.svg';
import robot from 'src/image/new_icons/robot.svg';
import teleport from 'src/image/new_icons/teleport.svg';
import { routes } from 'src/routes';
import { Networks } from 'src/types/networks';

// Core menu items — portal, robot, teleport, oracle
// Other apps (Docs, Nebula, Warp, Sphere, HFR, Mining, Senate, Cyberver, About, Studio)
// will be available as aos dapp add-ons to robot
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
  ];

  if (CHAIN_ID === Networks.BOSTROM) {
    listItemMenu.push({
      name: 'Portal',
      icon: portalGlow,
      largeIcon: portal,
      to: '/portal',
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

export default getMenuItems;

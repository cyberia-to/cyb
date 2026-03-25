import portal from 'images/space-pussy.svg';
import portalGlow from 'src/image/space-pussy-glow.svg';
import { CHAIN_ID } from 'src/constants/config';
import congress from 'src/image/new_icons/congress.svg';
import hfr from 'src/image/new_icons/hfr.svg';
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
      name: 'Studio',
      icon: require('./images/studio.png'),
      to: routes.studio.path,
      subItems: [],
    },
    {
      name: 'robot',
      icon: robot,
      to: '/settings',
      subItems: [
        { name: 'Drive', to: '/settings', icon: '🟥' },
        { name: 'Keys', to: '/settings/keys', icon: '🗝' },
        { name: 'Signer', to: '/settings/signer', icon: '🖋️' },
        { name: 'Node', to: '/settings/node', icon: '🟢' },
        { name: 'Hotkeys', to: '/settings/hotkeys', icon: '⌨️' },
        { name: 'LLM', to: '/settings/llm', icon: '👾' },
      ],
    },
    {
      name: 'AOS',
      icon: require('./images/aos.png'),
      to: '/aos',
      subItems: [],
    },
  ];

  if (CHAIN_ID === Networks.BOSTROM) {
    listItemMenu.splice(2, 0, {
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
      ],
    });
  }
  return listItemMenu.filter((item) => item);
};

export default getMenuItems;

import nebulaIcon from 'src/image/new_icons/nebula.svg';
import teleport from 'src/image/new_icons/teleport.svg';
import hfr from 'src/image/new_icons/hfr.svg';
import robot from 'src/image/new_icons/robot.svg';
import shpere from 'src/image/new_icons/sphere.svg';
import senate from 'src/image/new_icons/senate.svg';

import portal from 'images/space-pussy.svg';
import oracle from 'src/image/new_icons/oracle.svg';
import warp from 'src/image/new_icons/warp.svg';
import congress from 'src/image/new_icons/congress.svg';
import { routes } from 'src/routes';
import { Networks } from 'src/types/networks';
import { cybernetRoutes } from 'src/features/cybernet/ui/routes';
import { CHAIN_ID } from 'src/constants/config';
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
      subItems: [
        // {
        //   name: 'space-pussy',
        //   to: `${routes.sphere.path}/${Networks.SPACE_PUSSY}`,
        //   icon: require('./images/astronaut.png'),
        // },
      ],
    },
    { name: 'HFR', icon: hfr, to: '/hfr', subItems: [] },
    // { name: 'Lifeforms', to: '/contracts', subItems: [] },
    // {
    //   name: 'Hub',
    //   to: '/search/hub',
    //   icon: hub,
    //   subItems: [
    //     { name: 'Networks', to: '/networks' },
    //     { name: 'Add network', to: '/networks/add' },
    //   ],
    // },
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
              // not good, fix
              matchPathname: cybernetRoutes.subnet.path.replace(
                ':nameOrUid',
                'board'
              ),
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
      icon: require('./images/studio.png'),
      to: routes.studio.path,
      subItems: [],
    },
    // {
    //   name: 'Help',
    //   icon: zhdun,
    //   to: '/help',
    //   subItems: [
    //     {
    //       name: 'Guide',
    //       to: '/ipfs/QmRumrGFrqxayDpySEkhjZS1WEtMyJcfXiqeVsngqig3ak',
    //     },
    //     { name: 'story', to: '/genesis' },
    //     {
    //       name: 'vision',
    //       to: '/ipfs/QmXzGkfxZV2fzpFmq7CjAYsYL1M581ZD4yuF9jztPVTpCn',
    //     },
    //     {
    //       name: 'great web',
    //       to: '/ipfs/QmUamt7diQP54eRnmzqMZNEtXNTzbgkQvZuBsgM6qvbd57',
    //     },
    //     {
    //       name: 'vs govs',
    //       to: '/ipfs/QmPmJ4JwzCi82HZp7adtv5GVBFTsKF5Yoy43wshHH7x3ty',
    //     },
    //     {
    //       name: 'vs corps',
    //       to: '/ipfs/QmQvKF9Jb6QKmsqHJzEZJUfcbB9aBBKwa5dh3pMxYEj7oi',
    //     },
    //     {
    //       name: 'roadmap',
    //       to: '/ipfs/QmSBYCCYFNfHNQD7MWm4zBaNuztMaT2KghA2SbeZZm9vLH',
    //     },
    //     {
    //       name: 'distribution',
    //       to: '/ipfs/QmVPgNeay23Ae5itAamMcr4iEAUKuhw5qD9U1zNqN4gpew',
    //     },
    //     {
    //       name: 'gift',
    //       to: '/ipfs/QmPAi1h1rwWnHkNnxnHZg28eGivpUK8wy8eciqoPSR4PHv',
    //     },
    //     {
    //       name: 'congress',
    //       to: '/network/bostrom/contract/bostrom1xszmhkfjs3s00z2nvtn7evqxw3dtus6yr8e4pw',
    //     },
    //   ],
    // },
  ];

  if (CHAIN_ID === Networks.BOSTROM) {
    listItemMenu.splice(2, 0, {
      name: 'Portal',
      icon: portal,
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
        {
          name: 'Buy',
          to: '/buy',
          icon: '🤑',
        },
        // { name: 'Release', to: '/release' },
      ],
    });
  }
  return listItemMenu.filter((item) => item);
};

export default getMenuItems;

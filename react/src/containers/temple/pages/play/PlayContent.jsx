import { Link } from 'react-router-dom';
import { ContainerGradientText } from '../../../../components';
import portal from '../../../../image/space-pussy.svg';
import hfr from '../../../../image/temple/hfr.png';
import hub from '../../../../image/temple/hub.png';
import nebulaIcon from '../../../../image/temple/nebula.png';
import oracle from '../../../../image/temple/oracle.png';
import robot from '../../../../image/temple/robot.png';
import senate from '../../../../image/temple/senate.png';
import shpere from '../../../../image/temple/shpere.png';
import teleport from '../../../../image/temple/teleport.png';
import temple from '../../../../image/temple/temple.png';
import warp from '../../../../image/temple/warp.png';
import zhdun from '../../../../image/temple/zhdun.png';
import styles from './styles.scss';

const playContentItem = [
  {
    icon: nebulaIcon,
    title: 'nebula',
    description: 'web3 app store',
    to: '/nebula',
  },
  {
    icon: robot,
    title: 'robot',
    description: 'your immortal friend',
    to: '/robot',
  },
  {
    icon: oracle,
    title: 'oracle',
    description: 'search & explore web3',
    to: '/oracle',
  },
  {
    icon: teleport,
    title: 'teleport',
    description: 'send, bridge, swap',
    to: '/teleport',
  },
  {
    icon: shpere,
    title: 'sphere',
    description: 'handy staking',
    to: '/sphere',
  },
  { icon: warp, title: 'warp', description: 'powerfull dex', to: '/warp' },
  { icon: hfr, title: 'hfr', description: 'energy miner', to: '/hfr' },
  {
    icon: senate,
    title: 'senate',
    description: 'intergalactic governance',
    to: '/senate',
  },
  { icon: hub, title: 'hub', description: 'gpu black hole', to: '/search/hub' },
  {
    icon: temple,
    title: 'temple',
    description: 'holly knowledge',
    to: '/',
  },
  {
    icon: portal,
    title: 'portal',
    description: 'onboarding booster',
    to: '/portal',
  },
  {
    icon: zhdun,
    title: 'zhdun',
    description: 'more to come',
    to: '/search/zhdun',
  },
];

function PlayContent() {
  return (
    <ContainerGradientText status="green">
      <div className={styles.containerPlayContent}>
        {playContentItem.map((item) => (
          <Link to={item.to} key={item.to}>
            <div className={styles.containerItemPlayContent}>
              <div className={styles.containteImgItemPlayContent}>
                <img
                  alt={item.title}
                  src={item.icon}
                  className={styles.containerItemPlayContentImg}
                />
              </div>
              <div className={styles.containerPlayContentContainerText}>
                <div className={styles.containerItemPlayContentTitle}>{item.title}</div>
                <div className={styles.containerItemPlayContentDsc}>{item.description}</div>
              </div>
            </div>
          </Link>
        ))}
      </div>
    </ContainerGradientText>
  );
}

export default PlayContent;

import { Link } from 'react-router-dom';
import { routes } from 'src/routes';
import { isPussyChain } from 'src/utils/chains/pussy';

import robot from 'src/image/new_icons/robot.svg';
import nebulaIcon from 'src/image/new_icons/nebula.svg';
import warp from 'src/image/new_icons/warp.svg';
import shpere from 'src/image/new_icons/sphere.svg';
import hfr from 'src/image/new_icons/hfr.svg';
import senate from 'src/image/new_icons/senate.svg';
import congress from 'src/image/new_icons/congress.svg';
import mining from 'src/image/new_icons/mining.svg';

import styles from './AOS.module.scss';

const apps = [
  { name: 'Robot', icon: robot, to: '/robot' },
  { name: 'Nebula', icon: nebulaIcon, to: '/nebula' },
  { name: 'Warp', icon: warp, to: '/warp' },
  { name: 'Sphere', icon: shpere, to: routes.sphere.path },
  { name: 'HFR', icon: hfr, to: '/hfr' },
  { name: 'Senate', icon: senate, to: '/senate' },
  { name: 'Miner', icon: mining, to: '/mining' },
  { name: 'About', icon: congress, to: routes.social.path },
  {
    name: 'Docs',
    icon: require('src/image/new_icons/docs.svg'),
    to: 'https://docs.cyb.ai',
    external: true,
  },
  ...(isPussyChain
    ? [{ name: 'Cyberver', icon: require('src/utils/appsMenu/images/cyberver.png'), to: '/cyberver' }]
    : [{ name: 'Cyberver', icon: require('src/image/new_icons/cyberver.svg'), to: 'https://spacepussy.ai/cyberver', external: true }]
  ),
];

function AOS() {
  return (
    <div className={styles.container}>
      <h2 className={styles.title}>Age of Superintelligence</h2>
      <div className={styles.grid}>
        {apps.map((app) =>
          app.external ? (
            <a
              key={app.name}
              href={app.to}
              target="_blank"
              rel="noopener noreferrer"
              className={styles.card}
            >
              <img src={app.icon} alt={app.name} className={styles.icon} />
              <span className={styles.name}>{app.name}</span>
            </a>
          ) : (
            <Link key={app.name} to={app.to} className={styles.card}>
              <img src={app.icon} alt={app.name} className={styles.icon} />
              <span className={styles.name}>{app.name}</span>
            </Link>
          )
        )}
      </div>
    </div>
  );
}

export default AOS;

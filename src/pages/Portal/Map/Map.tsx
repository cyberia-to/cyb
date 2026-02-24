import { Link } from 'react-router-dom';
import { MainContainer } from 'src/components';
import { Stars } from 'src/containers/portal/components';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import { routes } from 'src/routes';
import Map from './Map';
import styles from './Map.module.scss';

const linksConfig = [
  {
    name: 'oracle',
    to: routes.oracle.ask.getLink('cyber'),
    className: styles.oracle,
  },
  {
    name: 'portal',
    to: routes.portal.path,
    className: styles.portal,
  },
  {
    name: 'gift',
    to: routes.gift.path,
    className: styles.gift,
  },
  {
    name: 'cyb',
    to: routes.robot.path,
    className: styles.cyb,
  },
  {
    name: 'teleport',
    to: routes.teleport.path,
    className: styles.teleport,
  },
  {
    name: 'sphere',
    to: routes.sphere.path,
    className: styles.sphere,
  },
  {
    name: 'senate',
    to: '/senate',
    className: styles.senate,
  },
  { name: 'nebula', to: '/nebula', className: styles.nebula },
  {
    name: 'hfr',
    to: routes.hfr.path,
    className: styles.hfr,
  },

  {
    name: 'cyberlink',
    to: routes.oracle.learn.path,
    className: styles.cyberlink,
  },
  {
    name: 'tweet',
    to: routes.robot.path,
    className: styles.tweet,
  },
  {
    name: 'grid',
    to: routes.robot.routes.energy.path,
    className: styles.grid,
  },

  {
    name: 'boot',
    to: routes.oracle.ask.getLink('boot'),
    className: styles.boot,
  },
  {
    name: 'hydrogen',
    to: routes.oracle.ask.getLink('hydrogen'),
    className: styles.hydrogen,
  },
  {
    name: 'millivolt',
    to: routes.oracle.ask.getLink('millivolt'),
    className: styles.millivolt,
  },
  {
    name: 'milliampere',
    to: routes.oracle.ask.getLink('milliampere'),
    className: styles.milliampere,
  },

  {
    name: 'cyberlink2',
    to: routes.oracle.ask.getLink('cyberlink'),
    className: styles.cyberlink2,
  },
];

function MapLink({ className, to, name }) {
  return (
    <Link to={to} className={className}>
      {name}
    </Link>
  );
}

function Map() {
  useAdviserTexts({
    defaultText: 'learn to navigate cyb.ai, but know the cyberverse is deeper than you think',
  });

  return (
    <MainContainer>
      <Stars />
      <div className={styles.wrapper}>
        <img className={styles.mapImg} src="/images/cyb-map.png" alt="map" />

        {linksConfig.map((link) => (
          <MapLink key={link.to} {...link} />
        ))}
      </div>
    </MainContainer>
  );
}

export default Map;

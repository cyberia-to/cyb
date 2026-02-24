import { Helmet } from 'react-helmet';
import { useLocation } from 'react-router-dom';
import { CHAIN_ID } from 'src/constants/config';
import { PATTERN_CYBER } from 'src/constants/patterns';
import { routes } from 'src/routes';
import getMenuItems from 'src/utils/appsMenu/appsMenu';
import findApp from 'src/utils/findApp';
import styles from './AppName.module.scss';

function AppName() {
  let { pathname } = useLocation();
  const isRobot = pathname.includes('@') || pathname.includes('neuron/');
  const isOracle = pathname.includes('oracle');

  if (isRobot) {
    const pathnameArr = pathname.replace(/^\/|\/$/g, '').split('/');
    const findItem = pathnameArr[pathnameArr.length - 1];
    pathname =
      findItem.includes('@') || findItem.match(PATTERN_CYBER) ? routes.robot.path : findItem;
  }

  if (isOracle) {
    pathname = routes.oracle.path;
  }

  const value = findApp(getMenuItems(), pathname);

  const content = value[0]?.name || CHAIN_ID;

  return (
    <>
      <Helmet>
        <title>{content ? `${content.toLowerCase()} | cyb` : ''}</title>
      </Helmet>
      <span className={styles.wrapper}>{content}</span>
    </>
  );
}

export default AppName;

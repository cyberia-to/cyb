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

  // Extract username for avatar pages like /@mastercyb
  const avatarMatch = pathname.match(/^\/@([^/]+)/);

  if (isRobot && !avatarMatch) {
    const pathnameArr = pathname.replace(/^\/|\/$/g, '').split('/');
    const findItem = pathnameArr[pathnameArr.length - 1];
    pathname =
      findItem.includes('@') || findItem.match(PATTERN_CYBER) ? '/settings' : findItem;
  }

  if (isRobot && avatarMatch) {
    pathname = '/settings';
  }

  if (isOracle) {
    pathname = routes.oracle.path;
  }

  const value = findApp(getMenuItems(), pathname);

  const isAvatar = !!avatarMatch;
  const content = isAvatar ? `@${avatarMatch[1]}.moon` : value[0]?.name || CHAIN_ID;

  return (
    <>
      <Helmet>
        <title>{content ? `${content.toLowerCase()} | cyb` : ''}</title>
      </Helmet>
      <span className={isAvatar ? styles.wrapperFull : styles.wrapper}>{content}</span>
    </>
  );
}

export default AppName;

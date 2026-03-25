import cx from 'classnames';
import { useMemo, useState } from 'react';
import { Link, useLocation } from 'react-router-dom';
import SubMenu from 'src/components/appMenu/SubMenu/SubMenu';
import { CHAIN_ID } from 'src/constants/config';
import usePassportByAddress from 'src/features/passport/hooks/usePassportByAddress';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppSelector } from 'src/redux/hooks';
import { routes } from 'src/routes';
import useMediaQuery from '../../../../hooks/useMediaQuery';
import { selectNetworkImg } from '../../../../utils/utils';
import styles from './CurrentApp.module.scss';
import AppSideBar from './ui/AppSideBar/AppSideBar';
import ChainInfo from './ui/ChainInfo/ChainInfo';
import { menuButtonId } from './utils/const';
import findSelectAppByUrl from './utils/findSelectAppByUrl';

function CurrentApp() {
  const mediaQuery = useMediaQuery('(min-width: 768px)');
  const location = useLocation();
  const address = useAppSelector(selectCurrentAddress);
  const { passport } = usePassportByAddress(address);
  const [openMenu, setOpenMenu] = useState(false);

  const getRoute = useMemo(() => {
    const { pathname } = location;

    return findSelectAppByUrl(pathname, passport, address);
  }, [location, address, passport]);

  const toggleMenu = (newState: boolean) => {
    setOpenMenu(newState);
  };

  const closeMenu = () => {
    toggleMenu(false);
  };

  const toggleMenuFc = useMemo(() => () => toggleMenu(!openMenu), [openMenu, toggleMenu]);

  const isAvatar = /^\/@/.test(location.pathname);

  return (
    <>
      <div className={styles.buttonWrapper}>
        <Link
          id={menuButtonId}
          to={getRoute[0]?.to || routes.oracle.path}
          className={styles.networkBtn}
          onClick={(e) => {
            if (!mediaQuery) {
              const hasSubItems = getRoute[0]?.subItems?.length > 0;
              if (hasSubItems) {
                e.preventDefault();
                toggleMenu(!openMenu);
              }
              // no subItems — follow the link normally
            }
          }}
        >
          <img
            alt="cyb"
            src={getRoute[0]?.largeIcon || getRoute[0]?.icon || selectNetworkImg(CHAIN_ID)}
            className={cx(
              isAvatar ? styles.avatarImg : styles.networkBtnImg,
              { [styles.portalGlow]: getRoute[0]?.name === 'Portal' }
            )}
          />
        </Link>
        {mediaQuery && <ChainInfo />}
      </div>

      {getRoute?.[0] && (
        <AppSideBar
          menuProps={{
            isOpen: mediaQuery || openMenu,
            toggleMenu: toggleMenuFc,
            closeMenu,
          }}
        >
          <SubMenu selectedApp={getRoute[0]} closeMenu={closeMenu} />
        </AppSideBar>
      )}
    </>
  );
}

export default CurrentApp;

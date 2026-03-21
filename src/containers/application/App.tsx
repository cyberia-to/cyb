import { useEffect } from 'react';
import { matchPath, Outlet, useLocation } from 'react-router-dom';
import { MainContainer } from 'src/components';
import { useBackend } from 'src/contexts/backend/backend';
import { useDevice } from 'src/contexts/device';
import { PreviousPageProvider } from 'src/contexts/previousPage';
import { AdviserColors } from 'src/features/adviser/Adviser/Adviser';
import { useAdviser } from 'src/features/adviser/context';
import { cybernetRoutes } from 'src/features/cybernet/ui/routes';
import { getPassport } from 'src/features/passport/passports.redux';
import useSenseManager from 'src/features/sense/ui/useSenseManager';
import { setTimeHistoryRoute } from 'src/features/TimeHistory/redux/TimeHistory.redux';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import MainLayout from 'src/layouts/Main';
import { initPocket } from 'src/redux/features/pocket';
import { useAppDispatch } from 'src/redux/hooks';
import { routes } from 'src/routes';
import { initCyblog } from 'src/utils/logging/bootstrap';
import AdviserContainer from '../../features/adviser/AdviserContainer';
import { setFocus } from './Header/Commander/commander.redux';
import { mobileAllowedRoutes } from './mobileAllowedRoutes';
import styles from './styles.scss';
import UseDesktopVersionBlock from './UseDesktopVersionBlock/UseDesktopVersionBlock';

export const PORTAL_ID = 'portal';

initCyblog();

function App() {
  const dispatch = useAppDispatch();
  const address = useCurrentAddress();

  // const { community, communityLoaded } = useGetCommunity(address || null, {
  //   main: true,
  // });

  const location = useLocation();
  const adviserContext = useAdviser();
  useSenseManager();

  const { ipfsError } = useBackend();

  useEffect(() => {
    dispatch(initPocket());
  }, [dispatch]);

  useEffect(() => {
    if (!address) {
      return;
    }

    dispatch(
      getPassport({
        address,
      })
    );
  }, [address, dispatch]);

  // reset
  // useEffect(() => {
  //   if (communityLoaded) {
  //     dispatch(setCommunity(community));
  //   }
  // }, [communityLoaded, community, dispatch]);


  useEffect(() => {
    // tabs
    if (
      [cybernetRoutes.verse.path, routes.senate.routes.proposal.path].some((path) => {
        return matchPath(path, location.pathname);
      })
    ) {
      return;
    }

    window.scrollTo(0, 0);
  }, [location.pathname]);

  useEffect(() => {
    dispatch(setTimeHistoryRoute(location.pathname));
  }, [location.pathname, dispatch]);

  useEffect(() => {
    if (ipfsError && !location.pathname.includes('/drive')) {
      adviserContext.setAdviser(
        <p>
          Could not connect to the IPFS API <br />
          <Link to={routes.robot.routes.drive.path}>Go to ipfs page</Link>
        </p>,
        AdviserColors.red
      );

      adviserContext.setIsOpen(true);
    }
  }, [ipfsError, location.pathname, adviserContext.setAdviser, adviserContext.setIsOpen]);

  // chekEvangelism = () => {
  //   const { location } = this.props;
  //   const { search } = location;

  //   if (search.match(/thanks=/gm) && search.match(/thanks=/gm).length > 0) {
  //     const parsed = queryString.parse(search);
  //     console.log('parsed', parsed);
  //     localStorage.setItem('thanks', JSON.stringify(parsed.thanks));
  //   }
  // };

  // Notify user when wallet auto-locks after 15 min
  useEffect(() => {
    const handleAutoLock = () => {
      adviserContext.setAdviser(
        'Wallet locked after inactivity. Enter your password to continue',
        AdviserColors.yellow
      );
      adviserContext.setIsOpen(true);
    };

    window.addEventListener('__cyb_wallet_locked', handleAutoLock);
    return () => window.removeEventListener('__cyb_wallet_locked', handleAutoLock);
  }, [adviserContext.setAdviser, adviserContext.setIsOpen]);

  // initial focus on commander
  useEffect(() => {
    const isGraphPages = window.location.pathname.includes('/brain');

    if (!isGraphPages) {
      dispatch(setFocus(true));
    }
  }, [dispatch]);

  const { isMobile } = useDevice();

  const mobileAllowed =
    location.pathname.includes('@') ||
    mobileAllowedRoutes.some((path) => {
      return matchPath(path, location.pathname);
    });

  return (
    <PreviousPageProvider>
      <MainLayout>
        {/* not move portal order */}
        {(location.pathname.includes('/brain') ||
          location.pathname.includes('/oracle2') ||
          location.pathname.includes('/graph')) && <div id={PORTAL_ID} className={styles.portal} />}

        <AdviserContainer />

        {isMobile && !mobileAllowed ? (
          <MainContainer>
            <UseDesktopVersionBlock />
          </MainContainer>
        ) : (
          <Outlet />
        )}
      </MainLayout>
    </PreviousPageProvider>
  );
}

export default App;

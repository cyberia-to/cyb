import { useEffect, useMemo, useRef } from 'react';
import { Link, Outlet, matchPath, useLocation } from 'react-router-dom';

import MainLayout from 'src/layouts/Main';
import { initPocket } from 'src/redux/features/pocket';

import { useBackend } from 'src/contexts/backend/backend';
import { AdviserColors } from 'src/features/adviser/Adviser/Adviser';
import { useAdviser } from 'src/features/adviser/context';
import { getPassport } from 'src/features/passport/passports.redux';
import { routes } from 'src/routes';

import AdviserContainer from 'src/features/adviser/AdviserContainer';
import useSenseManager from 'src/features/sense/ui/useSenseManager';
import { useAppDispatch } from 'src/redux/hooks';
import { initCyblog } from 'src/utils/logging/bootstrap';

// eslint-disable-next-line unused-imports/no-unused-imports, @typescript-eslint/no-unused-vars

import { MainContainer } from 'src/components';
import NewVersionChecker from 'src/components/NewVersionChecker/NewVersionChecker';
import SignerModal, {
  SignerModalRef,
} from 'src/components/signer-modal/signer-modal';
import { useDevice } from 'src/contexts/device';
import { PreviousPageProvider } from 'src/contexts/previousPage';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import { cybernetRoutes } from 'src/features/cybernet/ui/routes';
import { setTimeHistoryRoute } from 'src/features/TimeHistory/redux/TimeHistory.redux';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import { signerModalHandler } from 'src/services/signer/signer-modal-handler';
import { setFocus } from './Header/Commander/commander.redux';
import { mobileAllowedRoutes } from './mobileAllowedRoutes';
import styles from './styles.scss';
import UseDesktopVersionBlock from './UseDesktopVersionBlock/UseDesktopVersionBlock';

export const PORTAL_ID = 'portal';

initCyblog();

function App() {
  const dispatch = useAppDispatch();
  const address = useCurrentAddress();

  const signerModalRef = useRef<SignerModalRef>(null);

  const location = useLocation();
  const adviserContext = useAdviser();
  useSenseManager();

  const { ipfsError } = useBackend();

  useEffect(() => {
    dispatch(initPocket());
  }, []);

  useEffect(() => {
    if (signerModalRef?.current) {
      signerModalHandler.setSignerModalRef(signerModalRef.current);
    }
  }, [signerModalRef]);

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

  useAdviserTexts({
    defaultText: useMemo(() => {
      return (
        <div>
          There are network issues 😔, part of functionality is currently
          disabled
          <br />
          <Link to={routes.social.path}>check socials</Link> for more info
        </div>
      );
      // 'indexer is in sync now, some data may be not fully available'
    }, []),
  });

  useEffect(() => {
    // tabs
    if (
      [cybernetRoutes.verse.path, routes.senate.routes.proposal.path].some(
        (path) => {
          return matchPath(path, location.pathname);
        }
      )
    ) {
      return;
    }

    window.scrollTo(0, 0);
  }, [location.pathname]);

  useEffect(() => {
    dispatch(setTimeHistoryRoute(location.pathname));
  }, [location.pathname, dispatch]);

  useEffect(() => {
    if (
      ipfsError &&
      !location.pathname.includes('/drive') &&
      !location.pathname.includes('/mining')
    ) {
      adviserContext.setAdviser(
        <p>
          Could not connect to the IPFS API <br />
          <Link to={routes.robot.routes.drive.path}>Go to ipfs page</Link>
        </p>,
        AdviserColors.red
      );

      adviserContext.setIsOpen(true);
    }
  }, [ipfsError, location.pathname]);

  // chekEvangelism = () => {
  //   const { location } = this.props;
  //   const { search } = location;

  //   if (search.match(/thanks=/gm) && search.match(/thanks=/gm).length > 0) {
  //     const parsed = queryString.parse(search);
  //     console.log('parsed', parsed);
  //     localStorage.setItem('thanks', JSON.stringify(parsed.thanks));
  //   }
  // };

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
      <NewVersionChecker />
      <MainLayout>
        <>
          {/* not move portal order */}
          {(location.pathname.includes('/brain') ||
            location.pathname.includes('/oracle2') ||
            location.pathname.includes('/graph')) && (
            <div id={PORTAL_ID} className={styles.portal} />
          )}

          <AdviserContainer />

          {isMobile && !mobileAllowed ? (
            <MainContainer>
              <UseDesktopVersionBlock />
            </MainContainer>
          ) : (
            <Outlet />
          )}
        </>
      </MainLayout>
      <SignerModal ref={signerModalRef} />
    </PreviousPageProvider>
  );
}

export default App;

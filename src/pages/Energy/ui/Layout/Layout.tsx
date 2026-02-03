import { Outlet } from 'react-router-dom';
import { Helmet } from 'react-helmet';
import { MainContainer } from 'src/components';
import { MoonAnimation, Stars } from 'src/containers/portal/components';
// import styles from './Layout.module.scss';

function Layout() {
  return (
    <MainContainer width="62%">
      <Stars />
      {/* <MoonAnimation /> */}

      <div style={{ zIndex: 1 }}>
        <Helmet>
          <title>energy | cyb</title>
        </Helmet>

        <Outlet />
      </div>
    </MainContainer>
  );
}

export default Layout;

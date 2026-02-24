import { Helmet } from 'react-helmet';
import { Outlet } from 'react-router-dom';
import { MainContainer } from 'src/components';
import styles from './Layout.module.scss';
import SettingsMenu from './SettingsMenu/SettingsMenu';

function Layout() {
  return (
    <MainContainer>
      <div className={styles.wrapper}>
        <Helmet>
          <title>setting | cyb</title>
        </Helmet>

        <SettingsMenu />

        <Outlet />
      </div>
    </MainContainer>
  );
}

export default Layout;

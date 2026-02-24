import { Outlet } from 'react-router-dom';
import { Stars } from 'src/containers/portal/components';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import TabListTeleport from '../components/tabList/TabList';
import styles from './Layout.module.scss';

function Layout() {
  useAdviserTexts({
    defaultText: 'welcome to teleport',
  });

  return (
    <div>
      <Stars />

      <header className={styles.header}>
        <TabListTeleport />
      </header>

      <Outlet />
    </div>
  );
}

export default Layout;

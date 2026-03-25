import { Outlet } from 'react-router-dom';
import styles from './Layout.module.scss';

function Layout() {
  return (
    <div className={styles.container}>
      <Outlet />
    </div>
  );
}

export default Layout;

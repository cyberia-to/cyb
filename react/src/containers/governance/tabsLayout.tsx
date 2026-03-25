import { Outlet } from 'react-router-dom';
import TabListGovernance from './tabList';

function Layout() {
  return (
    <div>
      <TabListGovernance />
      <Outlet />
    </div>
  );
}

export default Layout;

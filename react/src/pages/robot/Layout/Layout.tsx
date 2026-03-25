import { Outlet, useLocation } from 'react-router-dom';
import { MainContainer } from 'src/components';
import Loader2 from 'src/components/ui/Loader2';
import { useRobotContext } from '../robot.context';
import RobotHeader from './RobotHeader/RobotHeader';
import useMenuCounts from './useMenuCounts';
import WrappedActionBar from './WrappedActionBar';

function Layout() {
  const { address, isLoading, nickname, isOwner } = useRobotContext();

  const location = useLocation();

  const counts = useMenuCounts(address);

  const title = `robot ${nickname || address || ''}`;

  return (
    <MainContainer title={title}>
      {isLoading ? (
        <Loader2 />
      ) : (
        <>
          {!isOwner && <RobotHeader menuCounts={counts} />}
          <Outlet />

          {!location.pathname.includes('brain') && <WrappedActionBar />}
        </>
      )}
    </MainContainer>
  );
}

export default Layout;

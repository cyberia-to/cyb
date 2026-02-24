import { Navigate, Route, Routes } from 'react-router-dom';
import Loader2 from 'src/components/ui/Loader2';
import RoutedEnergy from 'src/containers/energy';
import TableDiscipline from 'src/containers/gol/table';
import Sigma from 'src/containers/sigma';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import TxsTable from 'src/pages/robot/_refactor/account/component/txsTable';
import { routes } from 'src/routes';
import Follows from './_refactor/account/tabs/Follows/Follows';
import FeedsTab from './_refactor/account/tabs/feeds/feeds';
import Heroes from './_refactor/account/tabs/heroes';
import Brain from './Brain/Brain';
import Karma from './Karma/Karma';
import Layout from './Layout/Layout';
import LayoutRoot from './Layout/LayoutRoot/Layout';
import RobotContextProvider, { useRobotContext } from './robot.context';
import SensePage from './SensePage';
import Soul from './Soul/Soul';
import UnderConstruction from './UnderConstruction/UnderConstruction';
import ZeroUser from './ZeroUser/ZeroUser';

function RobotRoutes() {
  const { address, isFetched } = useRobotContext();

  const newUser = isFetched && !address;

  useAdviserTexts({
    defaultText: `${!newUser ? 'my' : 'welcome to'} robot`,
  });

  if (!isFetched) {
    return <Loader2 />;
  }

  return (
    <Routes>
      <Route path="/" element={<Layout />}>
        {newUser ? (
          <>
            <Route index element={<ZeroUser />} />
            <Route path="*" element={<Navigate to="/robot" />} />
          </>
        ) : (
          <>
            <Route element={<LayoutRoot />}>
              <Route index element={<FeedsTab />} />
              <Route path="soul" element={<Soul />} />

              {/* energy */}
              <Route path="/grid" element={<Navigate to={routes.robot.routes.energy.path} />} />
              {['energy', 'energy/:pageId'].map((path) => (
                <Route key={path} path={path} element={<RoutedEnergy />} />
              ))}

              <Route path="swarm" element={<Follows />} />
              <Route path="security" element={<Heroes />} />
              <Route path="rights" element={<UnderConstruction />} />
              <Route path="karma" element={<Karma />} />
              <Route path="badges" element={<TableDiscipline />} />
            </Route>

            {/* should be for logined user, but without layout */}
            <Route path="brain/*" element={<Brain />} />
          </>
        )}

        <Route path="sigma" element={<Sigma />} />
        <Route path="time" element={<TxsTable />} />

        {['sense', 'sense/:senseId'].map((path) => (
          <Route key={path} path={path} element={<SensePage />} />
        ))}

        <Route path="*" element={<p>Page does not exist</p>} />
      </Route>
    </Routes>
  );
}

function Robot() {
  return (
    <RobotContextProvider>
      <RobotRoutes />
    </RobotContextProvider>
  );
}

export default Robot;

import {
  BrowserRouter,
  HashRouter,
  Link,
  Navigate,
  Route,
  Routes,
  useParams,
} from 'react-router-dom';
import ErrorBoundary from './components/ErrorBoundary/ErrorBoundary';
import App from './containers/application/App';
import Block from './containers/blok';
import BlockDetails from './containers/blok/blockDetails';
import ForceQuitter from './containers/forceGraph/forceQuitter';
import GovernanceRoutes from './containers/governance/GovernanceRoutes';
import Help from './containers/help';
import Home from './containers/home/home';
import Ipfs from './containers/ipfs/ipfs';
import Market from './containers/market';
import Mint from './containers/mint';
import Movie from './containers/movie';
import AOS from './pages/AOS/AOS';
import NodePage from './pages/Node/Node';
import Nebula from './containers/nebula';
import Objects from './containers/Objects';
import Oracle from './containers/oracle';
import ParamNetwork from './containers/parameters';
import PortalCitizenship from './containers/portal';
import PortalGift from './containers/portal/gift';
import MainPartal from './containers/portal/mainPortal';
import Release from './containers/portal/release';
import SigmaWrapper from './containers/sigma/SigmaWrapper';
import Story from './containers/story/story';
import Temple from './containers/temple/Temple';
import TestKeplr from './containers/testKeplre';
import TrollBoxx from './containers/trollBox';
import Txs from './containers/txs';
import TxsDetails from './containers/txs/txsDetails';
import Warp from './containers/warp/Warp';
import WarpDashboardPools from './containers/warp/WarpDashboardPools';
import { CodePage, Codes, ContractPage, DashboardPage } from './containers/wasm';

import StudioWrapper from './features/studio/StudioWrapper';
import Keys from './pages/Keys/Keys';
import OracleLanding from './pages/oracle/landing/OracleLanding';
import Learn from './pages/oracle/Learn/Learn';
import OracleLanding from './pages/oracle/landing/OracleLanding';
import Map from './pages/Portal/Map/Map';
import ToOracleAsk from './pages/redirects/ToOracleAsk';
import Robot from './pages/robot/Robot';
import Mining from './pages/Mining/Mining';
// import Cybernet from './features/cybernet/ui/Cybernet';
import FreestyleIde from './pages/robot/Soul/RuneEditor/FreestyleIde/FreestyleIde';
import Filtering from './pages/Settings/Filtering/Filtering';
import Settings from './pages/Settings/Settings';
import Sign from './pages/Sign/Sign';
import Social from './pages/Social/Social';
import Sphere from './pages/Sphere/Sphere';
import Teleport from './pages/teleport/Teleport';
import { routes } from './routes';
import BrainRoutes from './routing/Brain';

type WrappedRouterProps = {
  children: React.ReactNode;
};

function WrappedRouter({ children }: WrappedRouterProps) {
  return process.env.IPFS_DEPLOY ? (
    <HashRouter>{children}</HashRouter>
  ) : (
    <BrowserRouter>{children}</BrowserRouter>
  );
}

function PageNotExist() {
  return (
    <div
      style={{
        textAlign: 'center',
      }}
    >
      page does not exists
      <br />
      <Link to={routes.home.path}>Home</Link>
    </div>
  );
}

function CheckPassportPage() {
  const params = useParams();

  if (params.username?.includes('@')) {
    return <Robot />;
  }

  return <PageNotExist />;
}

function ValidatorsRedirect() {
  const { status } = useParams();
  return <Navigate to={`/sphere/${status}`} />;
}

function RedirectToRobot() {
  const params = useParams();
  return <Navigate to={`/neuron/${params.address}`} replace />;
}

function AppRouter() {
  return (
    <WrappedRouter>
        <Routes>
          <Route path={routes.home.path} element={<App />}>
            <Route index element={<OracleLanding />} />
            <Route path="/ide" element={<FreestyleIde />} />

            <Route
              path="/robot/*"
              element={
                <ErrorBoundary>
                  <Robot />
                </ErrorBoundary>
              }
            />
            <Route path="/ipfs" element={<Navigate to={routes.settings.path} />} />

            <Route path="/sign" element={<Sign />} />

            <Route path={routes.temple.path} element={<Temple />} />
            <Route path={routes.neuron.path} element={<Robot />} />

            <Route path={routes.oracle.learn.path} element={<Learn />} />

            <Route path="/oracle/stats" element={<Home />} />
            <Route path="/oracle-old" element={<Oracle />} />

            <Route path="/ipfs/:query" element={<ToOracleAsk />} />
            <Route path={routes.oracle.ask.path} element={<Ipfs />} />

            <Route path="/oracle" element={<Navigate to={routes.oracle.path} />} />

            <Route path="/search" element={<Navigate to={routes.oracle.path} />} />
            <Route path="/search/:query" element={<ToOracleAsk />} />

            <Route path={routes.mining.path} element={<Mining />} />

            <Route
              path="/senate/*"
              element={
                <ErrorBoundary>
                  <GovernanceRoutes />
                </ErrorBoundary>
              }
            />

            {/* old links - start */}
            <Route path="/halloffame" element={<Navigate to="/sphere" />} />
            <Route path="/halloffame/:status" element={<ValidatorsRedirect />} />
            <Route path="/mint" element={<Navigate to={routes.hfr.path} />} />
            {/* old links - end */}

            <Route
              path="/sphere/*"
              element={
                <ErrorBoundary>
                  <Sphere />
                </ErrorBoundary>
              }
            />
            {/* <Route path="/sphere/:chainId/*" element={<Sphere />} /> */}

            <Route path="/episode-1" element={<Story />} />
            <Route path="/quitter" element={<ForceQuitter />} />

            {BrainRoutes()}

            <Route path="network/bostrom">
              <Route path="tx" element={<Txs />} />
              <Route path="tx/:txHash" element={<TxsDetails />} />

              <Route path="contract/:address" element={<RedirectToRobot />} />
              <Route path="contract/:address/:tab" element={<RedirectToRobot />} />

              {/* <Route path="hero/:address/" element={<ValidatorsDetails />} />
            <Route path="hero/:address/:tab" element={<ValidatorsDetails />} /> */}
              <Route path="parameters" element={<ParamNetwork />} />
              <Route path="parameters/:param" element={<ParamNetwork />} />
              <Route path="blocks" element={<Block />} />
              <Route path="blocks/:idBlock" element={<BlockDetails />} />
            </Route>
            <Route path="/degenbox" element={<TrollBoxx />} />
            <Route path="/test" element={<TestKeplr />} />
            <Route path={routes.hfr.path} element={<Mint />} />
            <Route path="/token" element={<Market />} />
            <Route path="/token/:tab" element={<Market />} />
            <Route path="/particles" element={<Objects />} />

            <Route
              path="/teleport/*"
              element={
                <ErrorBoundary>
                  <Teleport />
                </ErrorBoundary>
              }
            />

            <Route path="/warp" element={<WarpDashboardPools />} />
            <Route path="/warp/:tab" element={<Warp />} />
            <Route path="/genesis" element={<Movie />} />
            <Route path="/aos" element={<AOS />} />
            <Route path="/node" element={<NodePage />} />
            <Route path="/citizenship" element={<PortalCitizenship />} />
            <Route path="/gift" element={<PortalGift />} />
            <Route path="/release" element={<Release />} />
            <Route path="/portal" element={<MainPartal />} />
            <Route path="/portal/map" element={<Map />} />

            {/* wasm */}
            <Route path="/libs" element={<Codes />} />
            <Route path="/libs/:codeId" element={<CodePage />} />
            <Route path="/contracts" element={<DashboardPage />} />
            <Route path="/contracts/:contractAddress" element={<ContractPage />} />

            <Route path="/help" element={<Help />} />

            <Route path="/sigma" element={<SigmaWrapper />} />

            <Route path="/nebula" element={<Nebula />} />

            {/* seems shouldn't be build
          {process.env.CHAIN_ID === Networks.SPACE_PUSSY && (
            <Route path="/cyberver/*" element={<Cybernet />} />
          )} */}

            <Route path="/keys" element={<Keys />} />

            <Route
              path="/settings/*"
              element={
                <ErrorBoundary>
                  <Settings />
                </ErrorBoundary>
              }
            />

            <Route path={routes.social.path} element={<Social />} />

            {['/studio', '/studio/:cid'].map((path) => (
              <Route key={path} path={path} element={<StudioWrapper />} />
            ))}

            <Route path="/restricted-content" element={<Filtering />} />

            {/* works as 404 also */}
            <Route path=":username/*" element={<CheckPassportPage />} />
            <Route path="*" element={<PageNotExist />} />
          </Route>
        </Routes>
    </WrappedRouter>
  );
}

export default AppRouter;

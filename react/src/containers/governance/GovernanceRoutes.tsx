import { Route, Routes } from 'react-router-dom';
import { MainContainer } from 'src/components';
import { routes } from 'src/routes';
import CreateProposal from './CreateProposal/CreateProposal';
import Governance from './governance';
import ProposalsDetail from './proposalsDetail';

// FIXME: replace(routes.senate.path, '/')
function GovernanceRoutes() {
  return (
    <MainContainer>
      <Routes>
        <Route
          path={routes.senate.path.replace(routes.senate.path, '/')}
          element={<Governance />}
        />
        <Route
          path={routes.senate.routes.proposal.path.replace(routes.senate.path, '/')}
          element={<ProposalsDetail />}
        />

        <Route
          path={routes.senate.routes.new.path.replace(routes.senate.path, '/')}
          element={<CreateProposal />}
        />
      </Routes>
    </MainContainer>
  );
}

export default GovernanceRoutes;

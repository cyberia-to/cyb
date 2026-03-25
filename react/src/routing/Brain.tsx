import { Navigate, Route, useParams } from 'react-router-dom';
import Brain from 'src/pages/Brain/Brain';
import { routes } from 'src/routes';

function RedirectToRobotBrain() {
  const params = useParams();
  return <Navigate to={`/neuron/${params.agent}/brain`} replace />;
}

function BrainRoutes() {
  return (
    <>
      <Route path={routes.brain.path} element={<Brain />} />
      <Route path="/graph" element={<Navigate to={routes.brain.path} />} />
      <Route path="/pgraph/:agent" element={<RedirectToRobotBrain />} />
    </>
  );
}

export default BrainRoutes;

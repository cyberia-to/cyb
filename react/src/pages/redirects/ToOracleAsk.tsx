import { Navigate, useParams } from 'react-router-dom';
import { routes } from 'src/routes';

function ToOracleAsk() {
  const { query } = useParams();

  if (!query) {
    return <Navigate to={routes.oracle.path} replace />;
  }

  return <Navigate to={routes.oracle.ask.getLink(query)} replace />;
}

export default ToOracleAsk;

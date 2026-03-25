import { Link } from 'react-router-dom';
import { routes } from 'src/routes';
import { trimString } from 'src/utils/utils';

function TxHash({ hash }: { hash: string }) {
  return <Link to={routes.txExplorer.getLink(hash)}>{trimString(hash, 6, 6)}</Link>;
}

export default TxHash;

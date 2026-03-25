import { Link } from 'react-router-dom';
import { Dots } from 'src/components';
import ContentIpfs from 'src/components/contentIpfs/contentIpfs';
import useParticleDetails from 'src/features/particle/useParticleDetails';
import Date from 'src/features/sense/ui/components/Date/Date';
import { routes } from 'src/routes';
import { LogItem } from '../type';
import styles from './LogItemContent.module.scss';

function LogItemContent({ logItem }: { logItem: LogItem }) {
  const { data, loading } = useParticleDetails(logItem.cid, {
    skip: Boolean(!logItem.cid),
  });

  let content;

  if (loading) {
    content = (
      <span>
        resolving particle <Dots />
      </span>
    );
  } else if (data) {
    content = <ContentIpfs details={data} cid={data.cid} content={data.content} search />;
  }

  return (
    <div className={styles.wrapper}>
      <Link className={styles.content} to={routes.oracle.ask.getLink(logItem.cid)}>
        {content}
      </Link>

      <Link className={styles.tx} to={routes.txExplorer.getLink(logItem.txhash)}>
        <Date timestamp={logItem.timestamp} timeOnly />
      </Link>
    </div>
  );
}

export default LogItemContent;

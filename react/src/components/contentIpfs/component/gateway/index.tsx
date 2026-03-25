import Iframe from 'src/components/Iframe/Iframe';
import styles from './steles.scss';

function GatewayContent({ url }: { url: string }) {
  return (
    <div className={styles.gatewayCantainer}>
      <Iframe url={url} />
    </div>
  );
}

export default GatewayContent;

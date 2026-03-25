import IframeReact from 'react-iframe';
import styles from './styles.scss';

type Props = {
  url: string;
  width?: string;
  height?: string;
};

function Iframe({ url, width = '100%', height = '100%' }: Props) {
  return (
    <IframeReact
      width={width}
      height={height}
      id="iframeCid"
      className={styles.iframe}
      url={url}
      sandbox={['allow-scripts', 'allow-same-origin']}
    />
  );
}

export default Iframe;

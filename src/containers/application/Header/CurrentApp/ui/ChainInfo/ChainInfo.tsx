import { useLocation } from 'react-router-dom';
import { BandwidthBar } from 'src/components';
import AppName from '../AppName/AppName';
import styles from './ChainInfo.module.scss';

function ChainInfo() {
  const { pathname } = useLocation();
  const isAvatar = /^\/@/.test(pathname);

  return (
    <div className={styles.containerInfoSwitch}>
      <AppName />
      {!isAvatar && (
        <div className={styles.containerBandwidthBar}>
          <BandwidthBar tooltipPlacement="bottom" />
        </div>
      )}
    </div>
  );
}

export default ChainInfo;

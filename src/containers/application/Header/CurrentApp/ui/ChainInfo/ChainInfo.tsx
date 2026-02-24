import { BandwidthBar } from 'src/components';
import AppName from '../AppName/AppName';
import styles from './ChainInfo.module.scss';

function ChainInfo() {
  return (
    <div className={styles.containerInfoSwitch}>
      <AppName />
      <div className={styles.containerBandwidthBar}>
        <BandwidthBar tooltipPlacement="bottom" />
      </div>
    </div>
  );
}

export default ChainInfo;

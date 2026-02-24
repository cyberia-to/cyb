import { useMemo } from 'react';
import { ContainerGradientText } from '../../../../components/containerGradient/ContainerGradient';
import { dhm } from '../../../../utils/utils';
import { GIFT_ICON } from '../../utils';
import styles from './styles.scss';

function ItemRow({ value, title }) {
  return (
    <div className={styles.containerItemRow}>
      <div>{title}</div>
      <div>{value}</div>
    </div>
  );
}

function NextUnfreeze({ timeNext = '', readyRelease }) {
  try {
    const useTimeNext = useMemo(() => {
      if (timeNext !== null) {
        return <span className={styles.colorTextStatusTimeUnfreeze}>{dhm(timeNext)}</span>;
      }

      if (readyRelease && readyRelease !== null) {
        return <span className={styles.colorTextStatusTimeReady}>ready {GIFT_ICON}</span>;
      }

      return '';
    }, [timeNext, readyRelease]);
    return (
      <ContainerGradientText status="green">
        <div className={styles.containerNextUnfreeze}>
          <ItemRow value={useTimeNext} title="next unfreeze" />
          {/* <ItemRow
            value={`${formatNumber(readyRelease)} ${BOOT_ICON}`}
            title="ready to release"
          /> */}
        </div>
      </ContainerGradientText>
    );
  } catch (_error) {
    return (
      <ContainerGradientText status="green">
        <div className={styles.containerNextUnfreeze}>
          <ItemRow value="" title="next unfreeze" />
          {/* <ItemRow value={`0 ${BOOT_ICON}`} title="ready to release" /> */}
        </div>
      </ContainerGradientText>
    );
  }
}

export default NextUnfreeze;

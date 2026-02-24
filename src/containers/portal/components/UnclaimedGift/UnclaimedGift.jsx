import { ContainerGradientText } from '../../../../components/containerGradient/ContainerGradient';
import styles from './styles.scss';

function ItemRow({ value, title }) {
  return (
    <div className={styles.containerItemRow}>
      <div>{title}</div>
      <div>{value}</div>
    </div>
  );
}

function UnclaimedGift({ unClaimedGiftAmount = '' }) {
  try {
    return (
      <ContainerGradientText status="green">
        <div className={styles.containerUnclaimedGift}>
          <ItemRow value={unClaimedGiftAmount} title="unclaimed gift" />
        </div>
      </ContainerGradientText>
    );
  } catch (_error) {
    return (
      <ContainerGradientText status="green">
        <div className={styles.containerUnclaimedGift}>
          <ItemRow value="~" title="Unclaimed gift -" />
        </div>
      </ContainerGradientText>
    );
  }
}

// eslint-disable-next-line import/no-unused-modules
export default UnclaimedGift;

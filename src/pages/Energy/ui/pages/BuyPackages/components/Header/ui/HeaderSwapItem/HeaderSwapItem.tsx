import { useAppSelector } from 'src/redux/hooks';
import { ColorText, LinkWindowOsmo } from '../utils';
import styles from './HeaderSwapItem.module.scss';

function HeaderSwapItem() {
  const { swapResult } = useAppSelector((state) => state.energy);

  if (!swapResult) {
    return <ColorText text="waiting for transaction …" color="yellow" />;
  }

  if (swapResult.swapTx) {
    return (
      <span className={styles.wrapper}>
        <ColorText color="green" text="success" />
        <LinkWindowOsmo to={swapResult.swapTx} />
      </span>
    );
  }

  return null;
}

export default HeaderSwapItem;

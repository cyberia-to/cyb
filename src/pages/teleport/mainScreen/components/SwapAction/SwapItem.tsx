import cx from 'classnames';
import rectangle from 'images/rectangle.svg';
import { createSearchParams, Link } from 'react-router-dom';
import { DenomArr } from 'src/components';
import { SelectedPool } from '../../type';
import styles from './SwapItem.module.scss';

type Props = {
  item: SelectedPool;
};

type flexRow = {
  children: React.ReactNode;
  reverse?: boolean;
};

function FlexRow({ children, reverse }: flexRow) {
  return (
    <div
      className={cx(styles.containerRoweDenom, {
        [styles.containerRoweDenomReverse]: reverse,
      })}
    >
      {children}
    </div>
  );
}

function DenomsText({ item }: Props) {
  return (
    <FlexRow reverse={item.reverse}>
      <DenomArr denomValue={item.base_currency} onlyText tooltipStatusText={false} />
      -
      <DenomArr denomValue={item.target_currency} onlyText tooltipStatusText={false} />
    </FlexRow>
  );
}

function DenomsImg({ item }: Props) {
  return (
    <FlexRow reverse={item.reverse}>
      <DenomArr denomValue={item.base_currency} onlyImg tooltipStatusImg={false} size={30} />
      <img className={styles.denomsImgImg} src={rectangle} alt="img" />
      <DenomArr denomValue={item.target_currency} onlyImg tooltipStatusImg={false} size={30} />
    </FlexRow>
  );
}

function SwapItem({ item }: Props) {
  const queryParams = [item.base_currency, item.target_currency];

  if (item.reverse) {
    queryParams.reverse();
  }

  return (
    <Link
      to={{
        pathname: 'swap',
        search: createSearchParams({
          from: queryParams[0],
          to: queryParams[1],
        }).toString(),
      }}
      key={item.ticker_id}
      className={styles.containerSwapItem}
    >
      <DenomsImg item={item} />
      <DenomsText item={item} />
    </Link>
  );
}

export default SwapItem;

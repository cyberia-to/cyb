import cx from 'classnames';
import styles from './Dots.module.scss';

type DotsProps = {
  big?: boolean;
};

// eslint-disable-next-line import/prefer-default-export
export function Dots({ big }: DotsProps) {
  return (
    <div className={cx(styles.loaderDot, { [styles.loaderDotBig]: big })}>
      <span>.</span>
      <span>.</span>
      <span>.</span>
    </div>
  );
}

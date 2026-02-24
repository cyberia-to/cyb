import cx from 'classnames';
import { useEffect, useRef } from 'react';
import styles from './Dot.module.scss';

export enum DotColors {
  blue = 'blue',
  green = 'green',
  red = 'red',
  purple = 'purple',
  yellow = 'yellow',
}

type Props = {
  color: DotColors | keyof typeof DotColors;
  className?: string;
  animation?: boolean;
  size?: number;
};

function Dot({ color, className, animation, size = 4 }: Props) {
  const ref = useRef<HTMLLabelElement | null>(null);

  useEffect(() => {
    if (ref.current) {
      ref.current.style.setProperty('--size', `${size}px`);
    }
  }, [size]);

  return (
    <span
      ref={ref}
      className={cx(styles.dot, styles[`color_${color}`], className, {
        [styles.animation]: animation,
      })}
    />
  );
}

export default Dot;

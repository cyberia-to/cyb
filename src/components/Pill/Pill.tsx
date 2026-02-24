import cx from 'classnames';
import React from 'react';
import styles from './Pill.module.scss';

export type Props = {
  text: string | React.ReactNode;
  color?: 'white' | 'black' | 'blue' | 'red' | 'green' | 'yellow' | 'green-outline';
  image?: React.ReactNode;
  className?: string;
};

function Pill({ color = 'black', text, image, className }: Props) {
  return (
    <div
      className={cx(styles.pill, className, styles[color], {
        [styles['--withImage']]: image,
      })}
    >
      {image}
      {text}
    </div>
  );
}

export default Pill;

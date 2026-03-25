import cx from 'classnames';
import React from 'react';
import DisplayTitle from '../DisplayTitle/DisplayTitle';
import { ColorLamp, Colors } from '../types';
import styles from './Display.module.scss';

type Props = {
  children: React.ReactNode;

  /**
   * @deprecated use color props
   */
  status?: ColorLamp;

  noPaddingX?: boolean;
  noPaddingY?: boolean;
  noPadding?: boolean;

  sideSaber?: 'left' | 'right';

  color?: ColorLamp;
  title?: React.ReactElement<typeof DisplayTitle>;
  isVertical?: boolean;
};

function Display({
  children,
  isVertical,
  title,
  noPaddingX,
  noPaddingY,
  noPadding,
  sideSaber,
  color = Colors.GREEN,
  status,
}: Props) {
  const colorTemp = color || status;

  return (
    <div
      className={cx(styles.wrapper, styles[colorTemp], {
        [styles.vertical]: isVertical,
        [styles.noPaddingX]: noPaddingX || noPadding,
        [styles.noPaddingY]: noPaddingY || noPadding,
        [sideSaber ? styles[sideSaber] : undefined]: sideSaber,
      })}
    >
      {title && <header>{React.cloneElement(title, { inDisplay: true })}</header>}

      <div className={styles.inner}>{children}</div>
    </div>
  );
}

export default Display;

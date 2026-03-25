import BigNumber from 'bignumber.js';
import cx from 'classnames';
import React from 'react';
import getPrefixNumber from 'src/utils/getPrefixNumber';
import { formatNumber } from 'src/utils/utils';
import hydrogen from '../../image/hydrogen.svg';
import Tooltip from '../tooltip/tooltip';
import styles from './IconsNumber.module.scss';

enum TypesEnum {
  karma = 'karma',
  hydrogen = 'hydrogen',
  energy = 'energy',
  boot = 'boot',
  pussy = 'pussy',
}

// type Types = TypesEnum.karma | TypesEnum.hydrogen | TypesEnum.energy;

const icons = {
  [TypesEnum.karma]: '🔮',
  [TypesEnum.boot]: '🟢',
  [TypesEnum.hydrogen]: (
    <img
      height={17}
      style={{
        verticalAlign: 'middle',
      }}
      src={hydrogen}
      alt="hydrogen"
    />
  ),
  [TypesEnum.energy]: '🔋',
  [TypesEnum.pussy]: '🟣',
};

const POWER = new BigNumber(1000);

type Props = {
  type: keyof typeof TypesEnum;
  value: string | number;
  isVertical?: boolean;
};

export default function IconsNumber({ value, type, isVertical }: Props) {
  const prefix = getPrefixNumber(POWER.toNumber(), new BigNumber(value || 0).toNumber());

  const number = new BigNumber(value)
    .dividedBy(POWER.pow(prefix))
    .dp(0, BigNumber.ROUND_FLOOR)
    .toNumber();

  const i = new Array(prefix || 1).fill(icons[type]).map((el, idx) => {
    if (typeof el === 'object') {
      return React.cloneElement(el, { key: idx });
    }

    return <span key={idx}>{el}</span>;
  });

  const numberStyle = type === 'hydrogen' ? styles.hydrogenPlaceholder : '';

  return (
    <span className={styles.wrapper}>
      <span className={cx(styles.number, numberStyle)}>{number}</span>
      <Tooltip
        tooltip={
          <span className={styles.tooltipWrapper}>
            {formatNumber(value?.toLocaleString()?.replaceAll(',', ' ')) || 0} {icons[type]} {type}
          </span>
        }
      >
        <div className={cx(styles.icon, { [styles.vertical]: isVertical })}>{i}</div>
      </Tooltip>
    </span>
  );
}

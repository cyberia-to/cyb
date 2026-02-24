import BigNumber from 'bignumber.js';
import { useMemo } from 'react';
import { Dots } from 'src/components';
import getPrefixNumber from 'src/utils/getPrefixNumber';
import styles from './Level.module.scss';

const POWER = 1000;

enum LevelValue {
  energy = 'energy',
  sigma = 'sigma',
  karma = 'karma',
}

export type Props = {
  energy: number;
  sigma: number;
  karma: string;
};

function Level({ value }: { value: Props }) {
  const level = useMemo(() => {
    if (!value) {
      return <Dots />;
    }

    return Object.keys(LevelValue).reduce((acc: number, item) => {
      const prefix = getPrefixNumber(POWER, new BigNumber(value[item] || 0).toNumber());

      return new BigNumber(acc).plus(prefix).toNumber();
    }, 0);
  }, [value]);

  return (
    <span className={styles.text}>
      level <span className={styles.value}>{level}</span>
    </span>
  );
}

export default Level;

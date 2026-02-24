import cx from 'classnames';
import { useState } from 'react';
import Pill from 'src/components/Pill/Pill';
import { JsonRecord } from 'src/utils/localStorage';
import styles from './KeyItem.module.scss';

type Props = {
  name: string;
  value: JsonRecord[keyof JsonRecord];
  selected: boolean;
  selectKey: (name: string) => void;
};

function KeyItemSecrets({ name, value, selected, selectKey }: Props) {
  const [showValue, setShowValue] = useState(false);
  return (
    <div
      className={cx(styles.wrapper, {
        [styles.selected]: selected,
      })}
      onClick={() => selectKey(name)}
    >
      <div className={styles.imageWrapper}>
        <img src={require('./images/secrets.png')} alt="" />
      </div>

      <div className={styles.content}>
        secret <Pill color="white" text={name || 'noname'} /> <br />
        contains value{' '}
        <button
          className={cx(styles.pointer, styles.button)}
          onClick={() => setShowValue(!showValue)}
        >
          <Pill text={!showValue ? '***********' : value} />
        </button>
        <br />
        injects into <Pill color="green" text={'rune'} /> script scenarios in the app
        <br />
      </div>
    </div>
  );
}

export default KeyItemSecrets;

import BigNumber from 'bignumber.js';
import { CSSProperties } from 'react';
import { formatNumber } from '../../utils/utils';
import DenomArr from '../denom/denomArr';
import styles from './FormatNumberTokens.module.scss';

const getDecimal = (number: number) => {
  const nstring = new BigNumber(number).toString();
  const narray = nstring.split('.');
  let position = 3;
  if (number < 0.001) {
    // position = Math.floor(Math.abs(Math.log10(number))) + 2;
    position = 6;
  }
  const result = narray.length > 1 ? `.${narray[1].slice(0, position)}` : '';
  return result;
};

type Props = {
  text?: string;
  value: number;
  tooltipStatusImg?: boolean;
  styleValue?: CSSProperties;
  customText?: string;
  marginContainer?: string;
  styleContainer?: CSSProperties;
};

function FormatNumberTokens({
  text,
  value,
  tooltipStatusImg,
  styleValue,
  customText,
  marginContainer,
  styleContainer,
}: Props) {
  const decimal = getDecimal(value);
  return (
    <div
      style={{
        gridTemplateColumns: text || customText ? '1fr 20px' : '1fr',
        ...styleContainer,
      }}
      className={styles.containerFormatNumberTokens}
    >
      <div style={styleValue} className={styles.containerFormatNumberTokensNumber}>
        <span>{formatNumber(Math.floor(value))}</span>
        <span className={styles.containerFormatNumberTokensNumberDecimal}>{decimal}</span>
      </div>
      {text && (
        <div className={styles.containerFormatNumberTokensDenomImg}>
          <DenomArr
            marginImg="0 3px 0 0"
            denomValue={text}
            onlyImg
            marginContainer={marginContainer}
            tooltipStatusImg={tooltipStatusImg}
          />
        </div>
      )}
      {customText && !text && (
        <div className={styles.containerFormatNumberTokensDenomImg}>{customText}</div>
      )}
    </div>
  );
}

export default FormatNumberTokens;

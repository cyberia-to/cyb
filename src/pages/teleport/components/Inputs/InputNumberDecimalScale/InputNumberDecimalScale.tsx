import { useEffect, useState } from 'react';
import { InputNumber } from 'src/components';
import LinearGradientContainer, {
  Color,
} from 'src/components/LinearGradientContainer/LinearGradientContainer';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { $TsFixMeFunc } from 'src/types/tsfix';
import styles from './InputNumberDecimalScale.module.scss';

type Props = {
  id?: string;
  title: string;
  value: string;
  tokenSelect?: string;
  validAmount?: boolean;
  validAmountMessage?: boolean;
  validAmountMessageText?: string;
  warningAmount?: boolean;
  warningAmountText?: string;
  autoFocus?: boolean;
  availableAmount?: number;
  onValueChange: $TsFixMeFunc;
};

function InputNumberDecimalScale({
  validAmount,
  value,
  tokenSelect,
  onValueChange,
  title,
  validAmountMessage,
  availableAmount,
  validAmountMessageText,
  warningAmount,
  warningAmountText,
  ...props
}: Props) {
  const { tracesDenom } = useIbcDenom();
  const [fixed, setFixed] = useState(false);

  useEffect(() => {
    if (tracesDenom && tokenSelect) {
      const [{ coinDecimals }] = tracesDenom(tokenSelect);
      if (coinDecimals > 0) {
        setFixed(true);
        return;
      }
    }
    setFixed(false);
  }, [tracesDenom, tokenSelect]);

  if (validAmountMessage) {
    return (
      <div className={styles.containerAvailableAmount}>
        <LinearGradientContainer color={Color.Black} title={title}>
          <div className={styles.containerValue}>{validAmountMessageText}</div>
        </LinearGradientContainer>
      </div>
    );
  }

  let inputColor: Color;
  if (validAmount || value.length === 0) {
    inputColor = Color.Red;
  } else if (warningAmount) {
    inputColor = Color.Yellow;
  } else {
    inputColor = Color.Green;
  }

  return (
    <>
      <InputNumber
        maxValue={availableAmount}
        value={value}
        onChange={onValueChange}
        title={title}
        color={inputColor}
        fixedDecimalScale={fixed}
        {...props}
      />
      {warningAmount && warningAmountText && (
        <div style={{ color: 'var(--yellow)', fontSize: '0.75rem', marginTop: 4, textAlign: 'center' }}>
          {warningAmountText}
        </div>
      )}
    </>
  );
}

export default InputNumberDecimalScale;

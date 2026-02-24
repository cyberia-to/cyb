import { useState } from 'react';
import { NumericFormat } from 'react-number-format';
import { Color } from 'src/components/LinearGradientContainer/LinearGradientContainer';
import Input from '../Input';

type Props = {
  value: string | number;
  width?: string;
  onChange: Props['onValueChange'];
  // TODO: delete
  onValueChange?: (val: string, event: any) => void;
  id?: string;
  title?: string;
  color?: Color;
  fixedDecimalScale?: boolean;
  maxValue?: number;
  min?: number;
  autoFocus?: boolean;
};

function InputNumber({
  value,
  onValueChange,
  fixedDecimalScale,
  onChange,
  maxValue,
  min,
  autoFocus,
  ...props
}: Props) {
  const [focused, setFocused] = useState(false);

  return (
    <NumericFormat
      value={Number(value)}
      onValueChange={(values, sourceInfo) => {
        onValueChange?.(values.value, sourceInfo.event);
        onChange?.(values.value, sourceInfo.event);
      }}
      valueIsNumericString
      allowLeadingZeros
      customInput={Input}
      isAllowed={(values) => {
        const { floatValue } = values;
        if (typeof floatValue === 'number' && maxValue) {
          return floatValue <= maxValue;
        }
        return true;
      }}
      maxValue={maxValue}
      min={min}
      autoFocus={autoFocus}
      thousandsGroupStyle="thousand"
      thousandSeparator=" "
      decimalScale={3}
      fixedDecimalScale={fixedDecimalScale}
      onFocus={() => setFocused(true)}
      onBlur={() => setFocused(false)}
      autoComplete="off"
      focusedProps={focused}
      {...props}
    />
  );
}

export default InputNumber;

import { ComponentProps, Dispatch, SetStateAction } from 'react';
import { Input } from 'src/components';

type InputProps = ComponentProps<typeof Input>;
interface MnemonicInputProps {
  index: number;
  values: Record<number, string>;
  isTouched: boolean;

  onBlurFunc: InputProps['onBlurFnc'];
  setValues: Dispatch<SetStateAction<Record<number, string>>>;
}

export default function MnemonicInput({
  index,
  values,
  isTouched,
  setValues,
  onBlurFunc,
}: MnemonicInputProps) {
  return (
    <Input
      title={`${index + 1}`}
      error={isTouched && !values[index] ? `${index} is missing` : undefined}
      value={values[index] || ''}
      onChange={(e) => {
        setValues((prev) => ({ ...prev, [index]: e.target.value }));
      }}
      onBlurFnc={onBlurFunc}
    />
  );
}

import { ClipboardEvent, ComponentProps, useCallback } from 'react';
import { Input } from 'src/components';

type InputProps = ComponentProps<typeof Input>;
interface MnemonicInputProps {
  index: number;
  values: Record<number, string>;
  isTouched: boolean;

  onBlurFunc: InputProps['onBlurFnc'];
  onWordsDetected: (words: string[], startIndex: number) => void;
  onSingleChange: (index: number, value: string) => void;
}

export default function MnemonicInput({
  index,
  values,
  isTouched,
  onBlurFunc,
  onWordsDetected,
  onSingleChange,
}: MnemonicInputProps) {
  // Handle paste: detect multi-word and delegate to parent
  const handlePaste = useCallback(
    (e: ClipboardEvent<HTMLInputElement>) => {
      const paste = (e.clipboardData || (window as any).clipboardData)?.getData('text');
      if (!paste) return;

      const words = paste.trim().split(/\s+/);
      if (words.length > 1) {
        e.preventDefault();
        onWordsDetected(words, index);
      }
    },
    [index, onWordsDetected]
  );

  // Handle change: detect multi-word paste via onChange (Android fallback)
  const handleChange = useCallback(
    (e: { target: { value: string } }) => {
      const val = e.target.value;
      const words = val.trim().split(/\s+/);
      if (words.length > 1) {
        onWordsDetected(words, index);
      } else {
        onSingleChange(index, val);
      }
    },
    [index, onWordsDetected, onSingleChange]
  );

  return (
    <Input
      title={`${index + 1}`}
      error={isTouched && !values[index] ? `${index} is missing` : undefined}
      value={values[index] || ''}
      onChange={handleChange}
      onPaste={handlePaste}
      onBlurFnc={onBlurFunc}
    />
  );
}

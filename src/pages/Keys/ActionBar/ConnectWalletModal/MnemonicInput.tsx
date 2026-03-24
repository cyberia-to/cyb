import { ClipboardEvent, ComponentProps, useCallback } from 'react';
import { Input } from 'src/components';

type InputProps = ComponentProps<typeof Input>;
interface MnemonicInputProps {
  index: number;
  values: Record<number, string>;
  isTouched: boolean;
  showWords?: boolean;

  onBlurFunc: InputProps['onBlurFnc'];
  onWordsDetected: (words: string[], startIndex: number) => void;
  onSingleChange: (index: number, value: string) => void;
}

export default function MnemonicInput({
  index,
  values,
  isTouched,
  showWords = false,
  onBlurFunc,
  onWordsDetected,
  onSingleChange,
}: MnemonicInputProps) {
  const handlePaste = useCallback(
    (e: ClipboardEvent<HTMLInputElement>) => {
      const paste = e.clipboardData?.getData('text');
      if (!paste) return;

      const words = paste.trim().split(/\s+/);
      if (words.length > 1) {
        e.preventDefault();
        onWordsDetected(words, index);
        navigator.clipboard.writeText('').catch(() => {});
      }
    },
    [index, onWordsDetected]
  );

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
      type={showWords ? 'text' : 'password'}
      error={undefined}
      value={values[index] || ''}
      onChange={handleChange}
      onPaste={handlePaste}
      onBlurFnc={onBlurFunc}
      spellCheck={false}
      autoComplete="off"
      autoCorrect="off"
      autoCapitalize="off"
    />
  );
}

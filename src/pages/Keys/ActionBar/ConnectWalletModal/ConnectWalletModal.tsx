import { ClipboardEventHandler, useCallback, useEffect, useState } from 'react';
import { Button, Input } from 'src/components';
import Dropdown from 'src/components/Dropdown/Dropdown';
import Modal from 'src/components/modal/Modal';
import MnemonicInput from './MnemonicInput';

import * as styles from './ConnectWalletModal.style';

const columnsIndexes12 = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
const columnsIndexes24 = [
  0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21,
  22, 23,
];
const columns = { '12': columnsIndexes12, '24': columnsIndexes24 };
const dropdownOptions = [
  { label: '12', value: '12' },
  { label: '24', value: '24' },
];

interface ConnectWalletModalProps {
  onAdd(name: string, mnemonic: string): void | Promise<void>;
  onCancel(): void;
}

export default function ConnectWalletModal({
  onAdd,
  onCancel,
}: ConnectWalletModalProps) {
  const [mnemonicsLength, setMnemonicsLength] =
    useState<keyof typeof columns>('12');
  const [columnsIndexes, setColumnsIndexes] = useState(columnsIndexes12);
  const [values, setValues] = useState<Record<number, string>>(
    columnsIndexes.reduce((acc, index) => ({ ...acc, [index]: '' }), {})
  );
  const [name, setName] = useState('');
  const [isTouched, setIsTouched] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Best-effort cleanup on unmount — setState during unmount is a no-op in React 18,
  // string values remain in fiber tree until GC. Same inherent JS limitation as MetaMask/Keplr.
  useEffect(() => {
    return () => {
      setValues({});
      setName('');
    };
  }, []);

  const distributeWords = useCallback((words: string[], startIndex = 0) => {
    const targetLength: keyof typeof columns = words.length + startIndex > 12 ? '24' : '12';
    const targetCount = Number(targetLength);

    if (targetLength !== mnemonicsLength) {
      setMnemonicsLength(targetLength);
      setColumnsIndexes(columns[targetLength]);
    }

    setValues((prev) => {
      const newValues: Record<number, string> = {};
      for (let i = 0; i < targetCount; i++) {
        newValues[i] = i < startIndex ? (prev[i] || '') : '';
      }
      for (let i = 0; i < words.length && startIndex + i < targetCount; i++) {
        newValues[startIndex + i] = words[i].trim();
      }
      return newValues;
    });
  }, [mnemonicsLength]);

  const onMnemonicsPaste = useCallback<ClipboardEventHandler<HTMLDivElement>>(
    (event) => {
      event.preventDefault();

      const paste = event.clipboardData?.getData('text');
      if (!paste) return;

      const words = paste.trim().split(/\s+/);
      distributeWords(words);

      // Clear clipboard to prevent mnemonic leaking to clipboard managers
      navigator.clipboard.writeText('').catch(() => {});
    },
    [distributeWords]
  );

  useEffect(() => {
    setColumnsIndexes(columns[mnemonicsLength]);
  }, [mnemonicsLength]);

  const onSingleChange = useCallback((idx: number, val: string) => {
    setValues((prev) => ({ ...prev, [idx]: val }));
  }, []);

  const onInputBlurFunc = useCallback(() => {
    setIsTouched(true);
  }, []);

  const onAddClickWithValidation = useCallback(async () => {
    setIsTouched(true);
    setError(null);

    const filledCount = Object.values(values).filter(Boolean).length;
    const targetCount = Number(mnemonicsLength);

    if (!name) {
      return;
    }

    if (filledCount < targetCount) {
      return;
    }

    try {
      await onAdd(name, Object.values(values).join(' '));
    } catch {
      setError('Invalid seed phrase. Please check your words and try again.');
    }
  }, [onAdd, name, values, mnemonicsLength]);

  return (
    <Modal isOpen onPaste={onMnemonicsPaste} onClose={onCancel}>
      <div>
        <h3 style={styles.heading}>Enter your name</h3>
        <Input
          value={name}
          onChange={(e) => setName(e.target.value)}
          onBlurFnc={onInputBlurFunc}
          error={isTouched && !name ? 'Name is missing' : undefined}
        />
      </div>
      <div style={styles.wrapper}>
        <h3 style={styles.heading}>Enter or paste your seed phrase</h3>
        <div style={styles.dropdown}>
          <Dropdown
            value={mnemonicsLength}
            options={dropdownOptions}
            onChange={(v: string) => setMnemonicsLength(v as keyof typeof columns)}
          />
        </div>
      </div>
      <div style={styles.mnemonics}>
        {columnsIndexes.map((index) => (
          <MnemonicInput
            key={`mnemonic-input-${index}`}
            index={index}
            values={values}
            isTouched={isTouched}
            onBlurFunc={onInputBlurFunc}
            onWordsDetected={distributeWords}
            onSingleChange={onSingleChange}
          />
        ))}
      </div>
      {error && (
        <div style={{ color: '#ff4d4d', fontSize: '14px', marginTop: '8px', textAlign: 'center' }}>
          {error}
        </div>
      )}
      <div style={styles.buttons}>
        <Button onClick={onCancel}>Cancel</Button>
        <Button onClick={onAddClickWithValidation}>
          Add
        </Button>
      </div>
    </Modal>
  );
}

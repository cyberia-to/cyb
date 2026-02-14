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

  const onMnemonicsPaste = useCallback<ClipboardEventHandler<HTMLDivElement>>(
    (event) => {
      event.preventDefault();

      const paste = (event.clipboardData || window.clipboardData).getData(
        'text'
      );

      const words = paste.trim().split(/\s+/);
      console.log('[ConnectWalletModal] Pasted words:', words.length, words);
      setValues((prev) => {
        const newValues = { ...prev };
        for (let i = 0; i < words.length; i++) {
          newValues[i] = words[i].trim();
        }
        console.log('[ConnectWalletModal] New values after paste:', newValues);
        return newValues;
      });
    },
    []
  );

  useEffect(() => {
    setColumnsIndexes(columns[mnemonicsLength]);
  }, [mnemonicsLength]);

  const onInputBlurFunc = useCallback(() => {
    setIsTouched(true);
  }, []);

  const onAddClickWithValidation = useCallback(async () => {
    setIsTouched(true);

    const filledCount = Object.values(values).filter(Boolean).length;
    const targetCount = Number(mnemonicsLength);

    console.log('[ConnectWalletModal] Add clicked. name:', name, 'filled:', filledCount, '/', targetCount, 'values:', values);

    if (!name) {
      console.log('[ConnectWalletModal] Name is empty');
      return;
    }

    if (filledCount < targetCount) {
      console.log('[ConnectWalletModal] Not all words filled');
      return;
    }

    try {
      await onAdd(name, Object.values(values).join(' '));
    } catch (error) {
      console.error('[ConnectWalletModal] Failed to add mnemonics:', error);
    }
  }, [onAdd, name, values, mnemonicsLength]);

  return (
    <Modal isOpen onPaste={onMnemonicsPaste}>
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
            onChange={setMnemonicsLength as any}
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
            setValues={setValues}
            onBlurFunc={onInputBlurFunc}
          />
        ))}
      </div>
      <div style={styles.buttons}>
        <Button onClick={onCancel}>Cancel</Button>
        <Button onClick={onAddClickWithValidation}>
          Add
        </Button>
      </div>
    </Modal>
  );
}

import { RefObject, useCallback, useRef, useState } from 'react';
import { Input } from 'src/components';
import AddFileButton from 'src/components/buttons/AddFile/AddFile';
import { Color } from 'src/components/LinearGradientContainer/LinearGradientContainer';
import { useBackend } from 'src/contexts/backend/backend';
import styles from './InputMemo.module.scss';

type Props = {
  onChangeValue: (value: string, fileName?: string) => void;
  value: string;
  isTextarea?: boolean;
  title?: string;
};

function InputMemo({ onChangeValue, value, isTextarea, title = 'type public message' }: Props) {
  const { ipfsApi, isIpfsInitialized } = useBackend();
  const inputOpenFileRef = useRef<HTMLInputElement>(null);
  const [selectedFileName, setSelectedFileName] = useState<string | undefined>();

  const calculationIpfsTo = useCallback(
    async (file: File | undefined) => {
      if (!ipfsApi || !isIpfsInitialized || !file) {
        return;
      }

      const toCid = await ipfsApi.addContent(file);

      if (toCid) {
        onChangeValue(toCid, file.name);
      }
    },
    [ipfsApi, onChangeValue, isIpfsInitialized]
  );

  const onClickClear = () => {
    onChangeValue('');
    setSelectedFileName(undefined);
  };

  const showOpenFileDlg = () => {
    inputOpenFileRef.current!.click();
  };

  const onChangeInputFileRef = (files: RefObject<HTMLInputElement>) => {
    const file = files.current?.files![0];
    setSelectedFileName(file?.name);

    calculationIpfsTo(file);
  };

  return (
    <div className={styles.containerInputMemo}>
      <Input
        value={selectedFileName || value}
        onChange={(e) => onChangeValue(e.target.value)}
        title={title}
        color={Color.Pink}
        isTextarea={isTextarea}
        autoFocus
        className={styles.valueInputMemo}
      />
      <div className={styles.containerAddFileButton}>
        <input
          ref={inputOpenFileRef}
          onChange={() => onChangeInputFileRef(inputOpenFileRef)}
          type="file"
        />
        <AddFileButton
          isRemove={Boolean(value.length)}
          onClick={value.length ? onClickClear : showOpenFileDlg}
        />
      </div>
    </div>
  );
}

export default InputMemo;

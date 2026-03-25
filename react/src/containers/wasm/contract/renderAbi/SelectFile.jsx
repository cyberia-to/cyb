/* eslint-disable jsx-a11y/control-has-associated-label */
import { useEffect, useRef, useState } from 'react';
import AddFileButton from 'src/components/buttons/AddFile/AddFile';
import { toString as uint8ArrayToAsciiString } from 'uint8arrays/to-string';

function SelectFile({ stateCallback, text = 'Upload query schema' }) {
  const inputOpenFileRef = useRef(null);
  const [file, setFile] = useState(null);

  useEffect(() => {
    const updateState = async () => {
      if (stateCallback) {
        if (file !== null && file.type === 'application/json') {
          const bufferJson = new Uint8Array(await file.arrayBuffer());
          const stringJson = uint8ArrayToAsciiString(bufferJson);
          const jsonObj = JSON.parse(stringJson);
          stateCallback(jsonObj);
        }
      }
    };
    updateState();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [file, stateCallback]);

  const showOpenFileDlg = () => {
    inputOpenFileRef.current.click();
  };

  const onFilePickerChange = (files, e) => {
    const currentFile = files.current.files[0];
    setFile(currentFile);
    e.target.value = '';
  };

  const onClickClear = () => {
    setFile(null);
    stateCallback(null);
  };

  return (
    <div
      style={{
        display: 'flex',
        fontSize: '18px',
      }}
    >
      <div>{file?.name ? file.name : text}</div>
      <input
        ref={inputOpenFileRef}
        onChange={(e) => onFilePickerChange(inputOpenFileRef, e)}
        type="file"
        accept=".json"
        style={{ display: 'none' }}
      />
      <AddFileButton isRemove={file} onClick={file ? onClickClear : showOpenFileDlg} />
    </div>
  );
}

export default SelectFile;

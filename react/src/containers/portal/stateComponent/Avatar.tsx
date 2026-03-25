import { useBackend } from 'src/contexts/backend/backend';
import { ContainerGradient } from '../../../components';
import ContainerAvatar, { ButtonContainerAvatar } from '../components/avataIpfs/containerAvatar';

function Avatar({
  txs,
  valueNickname,
  upload,
  avatar,
  // setAvatarImg,
  // fncClearAvatar,
  inputOpenFileRef,
  onFilePickerChange,
  showOpenFileDlg,
}) {
  const { isIpfsInitialized } = useBackend();

  return (
    <ContainerGradient txs={txs} title="Moon Citizenship">
      <div
        style={{
          paddingLeft: '15px',
          height: '100%',
          display: 'grid',
          gridTemplateColumns: '1fr 1fr',
        }}
      >
        <div style={{ color: '#36D6AE' }}>{valueNickname}</div>

        <ContainerAvatar>
          <input
            ref={inputOpenFileRef}
            onChange={() => onFilePickerChange(inputOpenFileRef)}
            type="file"
            style={{ display: 'none' }}
          />
          <ButtonContainerAvatar
            uploadNew={upload}
            onClick={showOpenFileDlg}
            disabled={!isIpfsInitialized}
          >
            {upload ? 'upload avatar' : avatar}
          </ButtonContainerAvatar>
        </ContainerAvatar>
      </div>
    </ContainerGradient>
  );
}

export default Avatar;

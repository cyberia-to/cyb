import { useCallback, useEffect, useRef, useState } from 'react';
import { useBackend } from 'src/contexts/backend/backend';
import { useSigningClient } from 'src/contexts/signerClient';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';
import { Nullable } from 'src/types';
import { BtnGrd, Dots } from '../../components';
import { ActionBarContainer, ActionBarSteps } from './components';
import { CONTRACT_ADDRESS_PASSPORT } from './utils';

const STATE_INIT = 1;
const STATE_AVATAR = 15;
const STATE_AVATAR_IN_PROCESS = 15.1;

function ActionBarAddAvatar({ step, setStep, updateTxHash, citizenship }) {
  const { signingClient, signer } = useSigningClient();
  const { isIpfsInitialized, ipfsApi } = useBackend();

  const inputOpenFileRef = useRef();
  const [avatarIpfs, setAvatarIpfs] = useState<Nullable<string>>(null);
  const [avatarImg, setAvatarImg] = useState(null);

  const onFilePickerChange = (files) => {
    const file = files.current.files[0];
    setAvatarImg(file);
  };

  const showOpenFileDlg = () => {
    inputOpenFileRef.current.click();
  };

  useEffect(() => {
    const getPinAvatar = async () => {
      if (isIpfsInitialized && avatarImg !== null) {
        const toCid = await ipfsApi?.addContent(avatarImg);
        console.log('toCid', toCid);
        setAvatarIpfs(toCid);
      }
    };
    getPinAvatar();
  }, [isIpfsInitialized, ipfsApi, avatarImg]);

  const uploadAvatarImg = useCallback(async () => {
    if (signer && signingClient) {
      if (avatarIpfs !== null && citizenship) {
        try {
          const { nickname } = citizenship.extension;
          const msgObject = {
            update_avatar: {
              new_avatar: avatarIpfs,
              nickname,
            },
          };
          const [{ address }] = await signer.getAccounts();
          const executeResponseResult = await signingClient.execute(
            address,
            CONTRACT_ADDRESS_PASSPORT,
            msgObject,
            Soft3MessageFactory.fee(2.5),
            'cyber'
          );
          if (executeResponseResult.code === 0) {
            updateTxHash({
              status: 'pending',
              txHash: executeResponseResult.transactionHash,
            });
            setStep(STATE_AVATAR_IN_PROCESS);
          }

          if (executeResponseResult.code) {
            updateTxHash({
              txHash: executeResponseResult?.transactionHash,
              status: 'error',
              rawLog: executeResponseResult?.rawLog.toString(),
            });
            setStep(STATE_INIT);
          }
        } catch (error) {
          console.log('error', error);
          setStep(STATE_INIT);
        }
      } else {
        setStep(STATE_AVATAR);
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [avatarIpfs, signer, signingClient, citizenship, setStep, updateTxHash]);

  if (step === STATE_AVATAR) {
    return (
      <ActionBarSteps onClickBack={() => setStep(STATE_INIT)}>
        <input
          ref={inputOpenFileRef}
          onChange={() => onFilePickerChange(inputOpenFileRef)}
          type="file"
          style={{ display: 'none' }}
        />
        {avatarIpfs === null ? (
          <BtnGrd onClick={showOpenFileDlg} text="Upload new avatar" />
        ) : (
          <BtnGrd
            disabled={avatarIpfs === null}
            onClick={uploadAvatarImg}
            text={avatarIpfs == null ? <Dots /> : 'Upload'}
          />
        )}
      </ActionBarSteps>
    );
  }

  if (step === STATE_AVATAR_IN_PROCESS) {
    return (
      <ActionBarContainer>
        <BtnGrd pending />
      </ActionBarContainer>
    );
  }

  return null;
}

export default ActionBarAddAvatar;

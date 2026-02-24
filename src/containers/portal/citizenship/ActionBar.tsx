/* eslint-disable jsx-a11y/control-has-associated-label */
import { useCallback, useEffect, useState } from 'react';
import { connect } from 'react-redux';
import { useNavigate } from 'react-router-dom';
import NodeIsLoadingButton from 'src/components/btnGrd/NodeIsLoadingButton/NodeIsLoadingButton';
import { CHAIN_ID } from 'src/constants/config';
import { useBackend } from 'src/contexts/backend/backend';
import { useSigningClient } from 'src/contexts/signerClient';
import { toHex } from 'src/utils/encoding';
import { BtnGrd, Dots } from '../../../components';
import { setAccounts, setDefaultAccount } from '../../../redux/features/pocket';
import { LEDGER } from '../../../utils/config';
import { ActionBarSteps } from '../components';
import { steps } from './utils';

const {
  STEP_INIT,
  STEP_NICKNAME_CHOSE,
  STEP_NICKNAME_APROVE,
  STEP_NICKNAME_INVALID,
  STEP_RULES,
  STEP_AVATAR_UPLOAD,
  STEP_KEPLR_INIT,
  STEP_KEPLR_INIT_CHECK_FNC,
  STEP_KEPLR_INIT_INSTALLED,
  STEP_KEPLR_SETUP,
  STEP_KEPLR_CONNECT,
  STEP_CHECK_ADDRESS,
  STEP_CHECK_ADDRESS_CHECK_FNC,
  STEP_KEPLR_REGISTER,
  STEP_CHECK_GIFT,
  STEP_ACTIVE_ADD,
} = steps;

function ActionBar({
  step,
  setStep,
  setupNickname,
  checkNickname,
  valueNickname,
  avatarImg,
  uploadAvatarImg,
  avatarIpfs,
  onClickRegister,
  setAccountsProps,
  setDefaultAccountProps,
  checkAddressNetwork,
  registerDisabled,
  showOpenFileDlg,
  onClickSignMoonCode,
  signedMessage,
}) {
  const { signer } = useSigningClient();
  const navigate = useNavigate();
  const [checkAddressNetworkState, setCheckAddressNetworkState] = useState(false);

  const { isIpfsInitialized } = useBackend();

  const checkAddress = (obj, network, address) =>
    Object.keys(obj).filter((k) => obj[k][network] && obj[k][network].bech32 === address);

  const connectAccToCyber = async () => {
    let accounts = {};
    let key = 'Account 1';
    let dataPocketAccount = null;
    let valueObj = {};
    let pocketAccount = {};
    let defaultAccounts = null;
    let defaultAccountsKeys = null;
    const chainId = CHAIN_ID;

    let count = 1;
    if (signer) {
      const { bech32Address, pubKey, name } = await signer.keplr.getKey(chainId);
      const pk = toHex(new Uint8Array(pubKey));

      const localStoragePocketAccount = localStorage.getItem('pocketAccount');
      const localStorageCount = localStorage.getItem('count');
      if (localStorageCount !== null) {
        const dataCount = JSON.parse(localStorageCount);
        count = parseFloat(dataCount);
        key = `Account ${count}`;
      }
      localStorage.setItem('count', JSON.stringify(count + 1));
      if (localStoragePocketAccount !== null) {
        dataPocketAccount = JSON.parse(localStoragePocketAccount);
        valueObj = { ...dataPocketAccount };
      }

      const acc = checkAddress(valueObj, 'cyber', bech32Address);
      if (acc && acc.length > 0) {
        const activeKey = acc[0];
        key = activeKey;
        accounts = {
          ...valueObj[activeKey],
        };
      } else {
        accounts.cyber = {
          bech32: bech32Address,
          keys: 'keplr',
          pk,
          path: LEDGER.HDPATH,
          name,
        };
      }

      if (localStoragePocketAccount !== null) {
        if (Object.keys(accounts).length > 0) {
          pocketAccount = { [key]: accounts, ...dataPocketAccount };
        }
      } else {
        pocketAccount = { [key]: accounts };
      }
      if (Object.keys(pocketAccount).length > 0) {
        localStorage.setItem('pocketAccount', JSON.stringify(pocketAccount));
        const keys0 = Object.keys(pocketAccount)[0];
        localStorage.setItem('pocket', JSON.stringify({ [keys0]: pocketAccount[keys0] }));
        defaultAccounts = pocketAccount[keys0];
        defaultAccountsKeys = keys0;

        setAccountsProps(pocketAccount);
        setDefaultAccountProps(defaultAccountsKeys, defaultAccounts);

        setStep(STEP_CHECK_ADDRESS_CHECK_FNC);
      }
    }
  };

  const checkAddressNetworkOnClick = () => {
    if (checkAddressNetwork) {
      setCheckAddressNetworkState(true);
      checkAddressNetwork();
    }
  };

  const openInNewTab = (url) => {
    const newWindow = window.open(url, '_blank', 'noopener,noreferrer');
    if (newWindow) {
      newWindow.opener = null;
    }
  };

  const onClickInstallKeplr = useCallback(() => {
    if (!signer) {
      setStep(STEP_KEPLR_INIT_INSTALLED);
      openInNewTab('https://www.keplr.app');
    } else {
      setStep(STEP_KEPLR_SETUP);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [signer, openInNewTab, setStep]);

  useEffect(() => {
    if (step === STEP_KEPLR_REGISTER || step === STEP_ACTIVE_ADD) {
      setCheckAddressNetworkState(false);
    }
  }, [step]);

  const onClickSignMoonCodeCheckSined = useCallback(() => {
    if (signedMessage !== null) {
      setStep(STEP_KEPLR_REGISTER);
    } else {
      onClickSignMoonCode();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [signedMessage, onClickSignMoonCode, setStep]);

  if (step === STEP_INIT) {
    return (
      <ActionBarSteps>
        <BtnGrd onClick={() => setStep(STEP_NICKNAME_CHOSE)} text="start" />
      </ActionBarSteps>
    );
  }

  if (step === STEP_NICKNAME_CHOSE || step === STEP_NICKNAME_INVALID) {
    return (
      <ActionBarSteps onClickBack={() => setStep(STEP_INIT)}>
        <BtnGrd
          disabled={valueNickname === ''}
          onClick={() => checkNickname()}
          text="choose nickname"
        />
      </ActionBarSteps>
    );
  }

  if (step === STEP_NICKNAME_APROVE) {
    return (
      <ActionBarSteps onClickBack={() => setStep(STEP_NICKNAME_CHOSE)}>
        <BtnGrd onClick={() => setupNickname()} text="I like it" />
      </ActionBarSteps>
    );
  }

  if (step === STEP_AVATAR_UPLOAD) {
    return (
      <ActionBarSteps onClickBack={() => setStep(STEP_NICKNAME_APROVE)}>
        {!isIpfsInitialized ? (
          <NodeIsLoadingButton />
        ) : avatarIpfs === null ? (
          <BtnGrd onClick={showOpenFileDlg} text="upload avatar" />
        ) : (
          <BtnGrd
            disabled={avatarIpfs === null}
            onClick={uploadAvatarImg}
            text={avatarIpfs == null ? <Dots /> : 'upload'}
          />
        )}
      </ActionBarSteps>
    );
  }

  if (step === STEP_KEPLR_INIT || step === STEP_KEPLR_INIT_CHECK_FNC) {
    return (
      <ActionBarSteps onClickBack={() => setStep(STEP_AVATAR_UPLOAD)}>
        <BtnGrd
          onClick={() => onClickInstallKeplr()}
          text={!signer ? 'install Keplr' : 'I installed Keplr'}
        />
        {/* <Button onClick={() => setStep(STEP_KEPLR_SETUP)}>install Keplr</Button> */}
      </ActionBarSteps>
    );
  }

  if (step === STEP_KEPLR_INIT_INSTALLED) {
    return (
      <ActionBarSteps onClickBack={() => setStep(STEP_AVATAR_UPLOAD)}>
        <BtnGrd onClick={() => setStep(STEP_KEPLR_SETUP)} text="I installed Keplr" />
        {/* <Button onClick={() => setStep(STEP_KEPLR_SETUP)}>install Keplr</Button> */}
      </ActionBarSteps>
    );
  }

  if (step === STEP_KEPLR_SETUP) {
    return (
      <ActionBarSteps onClickBack={() => setStep(STEP_KEPLR_INIT)}>
        <BtnGrd onClick={() => setStep(STEP_KEPLR_CONNECT)} text="I created account" />
        {/* <Button onClick={() => setStep(STEP_KEPLR_CONNECT)}>
          I created account
        </Button> */}
      </ActionBarSteps>
    );
  }

  if (step === STEP_KEPLR_CONNECT) {
    return (
      <ActionBarSteps onClickBack={() => setStep(STEP_KEPLR_SETUP)}>
        {!signer ? (
          <BtnGrd onClick={() => document.location.reload(true)} text="update page" />
        ) : (
          <BtnGrd onClick={connectAccToCyber} text="connect" />
          // <Button onClick={() => connectAccToCyber()}>connect</Button>
        )}
      </ActionBarSteps>
    );
  }

  if (step === STEP_CHECK_ADDRESS) {
    return (
      <ActionBarSteps onClickBack={() => setStep(STEP_KEPLR_CONNECT)}>
        {/* check your bostrom address <Dots /> */}
        <BtnGrd onClick={() => setStep(STEP_CHECK_ADDRESS_CHECK_FNC)} text="check address" />
      </ActionBarSteps>
    );
  }

  if (step === STEP_CHECK_ADDRESS_CHECK_FNC) {
    return (
      <ActionBarSteps onClickBack={() => setStep(STEP_KEPLR_CONNECT)}>
        {/* check your bostrom address <Dots /> */}
        <BtnGrd
          disabled
          text={
            <>
              check your bostrom address <Dots />
            </>
          }
        />
      </ActionBarSteps>
    );
  }

  if (step === STEP_ACTIVE_ADD) {
    return (
      <ActionBarSteps onClickBack={() => setStep(STEP_KEPLR_CONNECT)}>
        {/* check your bostrom address <Dots /> */}
        <BtnGrd
          disabled={checkAddressNetworkState}
          onClick={checkAddressNetworkOnClick}
          text={
            checkAddressNetworkState ? (
              <>
                Activation <Dots />
              </>
            ) : (
              'activate address'
            )
          }
        />
      </ActionBarSteps>
    );
  }

  if (step === STEP_RULES) {
    return (
      <ActionBarSteps onClickBack={() => setStep(STEP_ACTIVE_ADD)}>
        <BtnGrd
          onClick={onClickSignMoonCodeCheckSined}
          text={signedMessage === null ? 'sign code' : 'next'}
        />
      </ActionBarSteps>
    );
  }

  if (step === STEP_KEPLR_REGISTER) {
    return (
      <ActionBarSteps onClickBack={() => setStep(STEP_RULES)}>
        <BtnGrd disabled={!registerDisabled} onClick={() => onClickRegister()} text="register" />
        {/* <Button onClick={() => onClickRegister()}>register</Button> */}
      </ActionBarSteps>
    );
  }

  if (step === STEP_CHECK_GIFT) {
    return (
      <ActionBarSteps>
        <BtnGrd onClick={() => navigate('/gift')} text="check gift" />
        {/* <Button onClick={() => navigate('/gift')}>check gift</Button> */}
      </ActionBarSteps>
    );
  }

  return null;
}

const mapDispatchprops = (dispatch) => {
  return {
    setDefaultAccountProps: (name, account) =>
      dispatch(
        setDefaultAccount({
          name,
          account,
        })
      ),
    setAccountsProps: (accounts) => dispatch(setAccounts(accounts)),
  };
};

export default connect(null, mapDispatchprops)(ActionBar);

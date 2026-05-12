/* eslint-disable */

import { Pane } from '@cybercongress/gravity';
import { useEffect, useState } from 'react';
import { useDispatch } from 'react-redux';
import { ActionBar, ConnectAddress, Dots, Input, TransactionError } from 'src/components';
import { CHAIN_ID } from 'src/constants/config';
import { PATTERN_CYBER } from 'src/constants/patterns';
import { useSigningClient } from 'src/contexts/signerClient';
import { addAddressPocket } from 'src/redux/features/pocket';
import { AccountValue } from 'src/types/defaultAccount';
import { LEDGER } from 'src/utils/config';
import { toHex } from 'src/utils/encoding';
import { encryptMnemonic } from 'src/utils/mnemonicCrypto';
import { getOfflineSigner, getOfflineSignerFromPrivateKey } from 'src/utils/offlineSigner';
import { setEncryptedMnemonic } from 'src/utils/utils';
import { useAdviser } from 'src/features/adviser/context';
import { AdviserColors } from 'src/features/adviser/Adviser/Adviser';
import { KEY_TYPE } from '../types';
import ActionBarSecrets from './actionBarSecrets';

const { STAGE_INIT, HDPATH, STAGE_ERROR } = LEDGER;

const STAGE_ADD_ADDRESS_USER = 2.1;
const STAGE_ADD_ADDRESS_OK = 2.2;
const STAGE_ADD_SECRETS = 100;

const _checkAddress = (obj, network, address) =>
  Object.keys(obj).some((k) => {
    if (obj[k][network]) {
      return obj[k][network].bech32 === address;
    }
  });

function ActionBarConnect({ addAddress, updateAddress, updateFuncActionBar, onClickBack }) {
  const { signer } = useSigningClient();
  const [stage, setStage] = useState(STAGE_INIT);
  const [valueInputAddres, setValueInputAddres] = useState('');
  const [selectMethod, setSelectMethod] = useState('');
  const selectNetwork = 'cyber';
  const [_addCyberAddress, setAddCyberAddress] = useState(false);
  const [validAddressAddedUser, setValidAddressAddedUser] = useState(true);

  // Secret flow state — useRef to avoid React DevTools exposure
  const pendingNameRef = useRef('');
  const pendingMnemonicRef = useRef('');
  const pendingImportModeRef = useRef<'mnemonic' | 'private-key'>('mnemonic');
  const [password, setPassword] = useState('');
  const [passwordConfirm, setPasswordConfirm] = useState('');
  const [passwordError, setPasswordError] = useState('');
  const [saving, setSaving] = useState(false);

  const dispatch = useDispatch();

  const clearState = () => {
    setStage(STAGE_INIT);
    setValueInputAddres('');
    setSelectMethod('');
    setAddCyberAddress(false);
    setValidAddressAddedUser(true);
    pendingNameRef.current = '';
    pendingMnemonicRef.current = '';
    pendingImportModeRef.current = 'mnemonic';
    setPassword('');
    setPasswordConfirm('');
    setPasswordError('');
    setSaving(false);
  };

  useEffect(() => {
    if (addAddress === false && stage === STAGE_ADD_ADDRESS_OK) {
      clearState();
    }
  }, [stage, addAddress, clearState]);

  useEffect(() => {
    if (valueInputAddres.match(PATTERN_CYBER)) {
      setValidAddressAddedUser(false);
    } else {
      setValidAddressAddedUser(true);
    }
  }, [valueInputAddres]);

  const connectAddress = () => {
    switch (selectMethod) {
      case KEY_TYPE.keplr:
        connectKeplr();
        break;
      case KEY_TYPE.secrets:
        onClickToggleSecrets();
        break;
      default:
        onClickAddAddressUser();
        break;
    }
  };

  const onClickAddAddressUser = () => {
    setStage(STAGE_ADD_ADDRESS_USER);
  };

  const onClickToggleSecrets = () => {
    setStage(STAGE_ADD_SECRETS);
  };

  const _onClickAddSecrets = () => {
    console.log('onClickAddSecrets');
  };

  const onClickAddAddressUserToLocalStr = async () => {
    const accounts = { bech32: valueInputAddres, keys: 'read-only' };

    setTimeout(() => {
      dispatch(addAddressPocket(accounts));
    }, 100);

    setStage(STAGE_ADD_ADDRESS_OK);

    clearState();
    if (updateAddress) {
      updateAddress();
    }
    if (updateFuncActionBar) {
      updateFuncActionBar();
    }
  };

  // Step 1: secret entered → ask for password
  const onSecretSubmit = (name: string, secret: string, mode: 'mnemonic' | 'private-key') => {
    pendingNameRef.current = name;
    pendingMnemonicRef.current = secret;
    pendingImportModeRef.current = mode;
    setStage(STAGE_SET_PASSWORD);
  };

      const accounts: AccountValue = {
        bech32: bech32Address,
        keys: 'keplr',
        pk,
        path: HDPATH,
        name,
      };

      setStage(STAGE_ADD_ADDRESS_OK);
      setTimeout(() => {
        dispatch(addAddressPocket(accounts));
      }, 100);

      clearState();
      if (updateAddress) {
        updateAddress();
      }
    }

    if (password !== passwordConfirm) {
      setPasswordError('Passwords do not match');
      return;
    }

    setSaving(true);
    try {
      const secret = pendingMnemonicRef.current;
      const isPrivateKey = pendingImportModeRef.current === 'private-key';

      const offlineSigner = isPrivateKey
        ? await getOfflineSignerFromPrivateKey(secret)
        : await getOfflineSigner(secret);

      if (offlineSigner) {
        const [{ address, pubkey: pubKey }] = await offlineSigner.getAccounts();
        const pk = toHex(pubKey);

        // Persist encrypted secret before setting signer —
        // if localStorage write fails, don't activate a non-persisted wallet
        const encrypted = await encryptMnemonic(secret, password);
        setEncryptedMnemonic(encrypted, address);
        activateWalletSigner(offlineSigner, secret);

        const accounts: AccountValue = {
          pk,
          keys: isPrivateKey ? 'private-key' : 'wallet',
          path: isPrivateKey ? undefined : HDPATH,
          name: pendingNameRef.current,
          bech32: address,
        };

        setStage(STAGE_ADD_ADDRESS_OK);
        setTimeout(() => {
          dispatch(addAddressPocket(accounts));
        }, 100);

        clearState();
        if (updateAddress) {
          updateAddress();
        }
        if (updateFuncActionBar) {
          updateFuncActionBar();
        }
      }
    } catch (err: any) {
      pendingMnemonicRef.current = '';
      setPassword('');
      setPasswordConfirm('');

      const isStorageError = err?.message?.includes('storage');
      setPasswordError(
        isStorageError
          ? 'Could not save wallet. Check browser storage settings'
          : 'Failed to import wallet. Check your secret and try again'
      );
    } finally {
      setSaving(false);
    }
  };

  const selectMethodFunc = (method) => {
    if (method !== selectMethod) {
      setSelectMethod(method);
    } else {
      setSelectMethod('');
    }
  };

  if (stage === STAGE_OPEN_MODAL) {
    return (
      <ConnectWalletModal
        onAdd={onSecretSubmit}
        onCancel={() => clearState()}
      />
    );
  }

  if (stage === STAGE_SET_PASSWORD) {
    return (
      <ActionBar
        button={{
          disabled: !password || !passwordConfirm || saving,
          onClick: onPasswordSubmit,
          text: 'Encrypt & Save',
        }}
        onClickBack={() => setStage(STAGE_OPEN_MODAL)}
      >
        <Pane display="flex" alignItems="center" justifyContent="center" flex={1} gap="10px">
          <Input
            width="200px"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="password"
            type="password"
            autoComplete="new-password"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck={false}
            autoFocus
          />
          <Input
            width="200px"
            value={passwordConfirm}
            onChange={(e) => setPasswordConfirm(e.target.value)}
            placeholder="confirm password"
            type="password"
            autoComplete="new-password"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck={false}
          />
          {passwordError && (
            <span style={{ color: '#ff4d4d', fontSize: '14px' }}>{passwordError}</span>
          )}
        </Pane>
      </ActionBar>
    );
  }

  if (stage === STAGE_INIT) {
    return (
      <ConnectAddress
        selectMethodFunc={selectMethodFunc}
        selectMethod={selectMethod}
        selectNetwork={selectNetwork}
        connectAddress={connectAddress}
        keplr={signer}
        onClickBack={onClickBack}
      />
    );
  }

  if (stage === STAGE_ADD_ADDRESS_USER) {
    return (
      <ActionBar
        button={{
          disabled: validAddressAddedUser,
          onClick: onClickAddAddressUserToLocalStr,
          text: 'Add address',
        }}
        onClickBack={() => setStage(STAGE_INIT)}
      >
        <Pane flex={1} justifyContent="center" alignItems="center" fontSize="18px" display="flex">
          put {selectNetwork} address:
          <Input
            width="250px"
            value={valueInputAddres}
            onChange={(e) => setValueInputAddres(e.target.value)}
            placeholder="address"
            autoFocus
          />
        </Pane>
      </ActionBar>
    );
  }

  if (stage === STAGE_ADD_SECRETS) {
    return <ActionBarSecrets onClickBack={() => setStage(STAGE_INIT)} />;
  }

  if (stage === STAGE_ADD_ADDRESS_OK) {
    return (
      <ActionBar>
        <Pane display="flex" alignItems="center">
          <Pane fontSize={20}>adding address</Pane>
          <Dots big />
        </Pane>
      </ActionBar>
    );
  }

  if (stage === STAGE_ERROR) {
    return (
      <TransactionError
        onClickBtn={() => clearState()}
        errorMessage="you have this address in your pocket"
      />
    );
  }

  return null;
}

export default ActionBarConnect;

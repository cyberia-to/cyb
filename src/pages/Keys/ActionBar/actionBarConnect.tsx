/* eslint-disable @typescript-eslint/no-explicit-any */

import { Pane } from '@cybercongress/gravity';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useDispatch } from 'react-redux';
import { ActionBar, ConnectAddress, Dots, Input, TransactionError } from 'src/components';
import { PATTERN_CYBER } from 'src/constants/patterns';
import { useSigningClient } from 'src/contexts/signerClient';
import { addAddressPocket } from 'src/redux/features/pocket';
import { AccountValue } from 'src/types/defaultAccount';
import { LEDGER } from 'src/utils/config';
import { toHex } from 'src/utils/encoding';
import { encryptMnemonic } from 'src/utils/mnemonicCrypto';
import { getOfflineSigner } from 'src/utils/offlineSigner';
import { isWebUSBSupported } from 'src/utils/ledgerSigner';
import { setEncryptedMnemonic } from 'src/utils/utils';
import { useAdviser } from 'src/features/adviser/context';
import { AdviserColors } from 'src/features/adviser/Adviser/Adviser';
import { KEY_TYPE } from '../types';
import ActionBarSecrets from './actionBarSecrets';
import ConnectWalletModal from './ConnectWalletModal/ConnectWalletModal';
import { ConnectMethod } from './types';

const { STAGE_INIT, HDPATH, STAGE_ERROR } = LEDGER;

const STAGE_ADD_ADDRESS_USER = 2.1;
const STAGE_ADD_ADDRESS_OK = 2.2;
const STAGE_OPEN_MODAL = 2.5;
const STAGE_SET_PASSWORD = 2.6;
const STAGE_LEDGER_WAITING = 2.7;
const STAGE_ADD_SECRETS = 100;

const PASSWORD_HINT =
  'Password protects your seed phrase. ' +
  'Use 8+ chars with mixed case, digits & symbols (e.g. "Cyb3r!net"), ' +
  'or 12+ chars of any kind. Weak example: "password" — don\'t do that';

function ActionBarConnect({ addAddress, updateAddress, updateFuncActionBar, onClickBack }) {
  const { activateWalletSigner, connectLedger } = useSigningClient();
  const { setAdviser } = useAdviser();
  const [stage, setStage] = useState(STAGE_INIT);
  const [valueInputAddres, setValueInputAddres] = useState('');
  const [connectMethod, setConnectMethod] = useState<ConnectMethod | ''>('');
  const selectNetwork = 'cyber';
  const [validAddressAddedUser, setValidAddressAddedUser] = useState(true);

  // Mnemonic flow state — useRef to avoid React DevTools exposure
  const pendingNameRef = useRef('');
  const pendingMnemonicRef = useRef('');
  const [password, setPassword] = useState('');
  const [passwordConfirm, setPasswordConfirm] = useState('');
  const [passwordError, setPasswordError] = useState('');
  const [saving, setSaving] = useState(false);

  const dispatch = useDispatch();

  const clearState = () => {
    setStage(STAGE_INIT);
    setValueInputAddres('');
    setConnectMethod('');
    setValidAddressAddedUser(true);
    pendingNameRef.current = '';
    pendingMnemonicRef.current = '';
    setPassword('');
    setPasswordConfirm('');
    setPasswordError('');
    setSaving(false);
  };

  // Cleanup on unmount — refs are cleared synchronously, setState is no-op in React 18
  useEffect(() => {
    return () => {
      pendingNameRef.current = '';
      pendingMnemonicRef.current = '';
    };
  }, []);

  useEffect(() => {
    if (addAddress === false && stage === STAGE_ADD_ADDRESS_OK) {
      clearState();
    }
  }, [stage, addAddress, clearState]);

  // Show password requirements in adviser when entering password stage
  useEffect(() => {
    if (stage === STAGE_SET_PASSWORD) {
      setAdviser(PASSWORD_HINT, AdviserColors.yellow);
    }
  }, [stage, setAdviser]);

  // Notify about Ledger availability when on connect screen
  useEffect(() => {
    if (stage === STAGE_INIT && !isWebUSBSupported()) {
      setAdviser(
        'Ledger requires Chrome, Edge, or the cyb.ai desktop app',
        AdviserColors.yellow
      );
    }
  }, [stage, setAdviser]);

  useEffect(() => {
    if (valueInputAddres.match(PATTERN_CYBER)) {
      setValidAddressAddedUser(false);
    } else {
      setValidAddressAddedUser(true);
    }
  }, [valueInputAddres]);

  const connectAddress = () => {
    switch (connectMethod) {
      case KEY_TYPE.secrets:
        onClickToggleSecrets();
        break;
      case KEY_TYPE.wallet:
        setStage(STAGE_OPEN_MODAL);
        break;
      case KEY_TYPE.ledger:
        onClickConnectLedger();
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

  const onClickConnectLedger = async () => {
    setStage(STAGE_LEDGER_WAITING);
    try {
      const { address, pubkey } = await connectLedger();
      const pk = toHex(pubkey);

      const accounts: AccountValue = {
        pk,
        keys: 'ledger',
        path: HDPATH,
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
    } catch (err: any) {
      setAdviser(
        `Could not connect to Ledger: ${err?.message || 'unknown error'}. Open the Cosmos app on your device and try again`,
        AdviserColors.red
      );
      setStage(STAGE_INIT);
    }
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

  // Step 1: mnemonic entered → ask for password
  const onMnemonicSubmit = (name: string, mnemonic: string) => {
    pendingNameRef.current = name;
    pendingMnemonicRef.current = mnemonic;
    setStage(STAGE_SET_PASSWORD);
  };

  // Step 2: password confirmed → encrypt & save
  const onPasswordSubmit = async () => {
    if (saving) return;
    setPasswordError('');

    if (password.length < 8) {
      setPasswordError('Password must be at least 8 characters');
      return;
    }

    // Require at least 3 of 4 character classes for passwords under 12 chars
    if (password.length < 12) {
      const classes = [/[a-z]/, /[A-Z]/, /[0-9]/, /[^a-zA-Z0-9]/];
      const matched = classes.filter((re) => re.test(password)).length;
      if (matched < 3) {
        setPasswordError(
          'Use uppercase, lowercase, digits and special characters (at least 3 of 4), or use 12+ characters'
        );
        return;
      }
    }

    if (password !== passwordConfirm) {
      setPasswordError('Passwords do not match');
      return;
    }

    setSaving(true);
    try {
      const mnemonic = pendingMnemonicRef.current;
      const offlineSigner = await getOfflineSigner(mnemonic);

      if (offlineSigner) {
        const [{ address, pubkey: pubKey }] = await offlineSigner.getAccounts();
        const pk = toHex(pubKey);

        // Persist encrypted mnemonic before setting signer —
        // if localStorage write fails, don't activate a non-persisted wallet
        const encrypted = await encryptMnemonic(mnemonic, password);
        setEncryptedMnemonic(encrypted, address);
        activateWalletSigner(offlineSigner, mnemonic);

        const accounts: AccountValue = {
          pk,
          keys: 'wallet',
          path: HDPATH,
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
    } catch (err) {
      pendingMnemonicRef.current = '';
      setPassword('');
      setPasswordConfirm('');
      setPasswordError('Failed to import wallet. Check your seed phrase and try again');
    } finally {
      setSaving(false);
    }
  };

  const selectMethodFunc = (method: ConnectMethod) => {
    if (method !== connectMethod) {
      setConnectMethod(method);
    } else {
      setConnectMethod('');
    }
  };

  if (stage === STAGE_OPEN_MODAL) {
    return (
      <ConnectWalletModal
        onAdd={onMnemonicSubmit}
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
            autoFocus
          />
          <Input
            width="200px"
            value={passwordConfirm}
            onChange={(e) => setPasswordConfirm(e.target.value)}
            placeholder="confirm password"
            type="password"
            autoComplete="new-password"
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
        selectMethod={connectMethod}
        selectNetwork={selectNetwork}
        connectAddress={connectAddress}
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

  if (stage === STAGE_LEDGER_WAITING) {
    return (
      <ActionBar onClickBack={() => setStage(STAGE_INIT)}>
        <Pane display="flex" alignItems="center">
          <Pane fontSize={20}>Connect your Ledger and open the Cosmos app</Pane>
          <Dots big />
        </Pane>
      </ActionBar>
    );
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

/* eslint-disable */
import { Pane } from '@cybercongress/gravity';
import { useContext, useEffect, useState } from 'react';
import { useDispatch } from 'react-redux';
import { ActionBar, ConnectAddress, Dots, Input, TransactionError } from 'src/components';
import { toHex } from '@cosmjs/encoding';
import { CHAIN_ID } from 'src/constants/config';
import { PATTERN_CYBER } from 'src/constants/patterns';
import { SignerClientContext } from 'src/contexts/signerClient';
import { addAddressPocket } from 'src/redux/features/pocket';
import { AccountValue } from 'src/types/defaultAccount';
import { LEDGER } from 'src/utils/config';
import { getOfflineSigner } from 'src/utils/offlineSigner';
import { KEY_TYPE } from '../types';
import ActionBarSecrets from './actionBarSecrets';
import ConnectWalletModal from './ConnectWalletModal/ConnectWalletModal';
import { ConnectMethod } from './types';
import { setMnemonic } from 'src/utils/utils';

const { STAGE_INIT, HDPATH, STAGE_ERROR } = LEDGER;

const STAGE_ADD_ADDRESS_USER = 2.1;
const STAGE_ADD_ADDRESS_OK = 2.2;
const STAGE_OPEN_MODAL = 2.5;
const STAGE_ADD_SECRETS = 100;

const _checkAddress = (obj, network, address) =>
  Object.keys(obj).some((k) => {
    if (obj[k][network]) {
      return obj[k][network].bech32 === address;
    }
  });

function ActionBarConnect({
  addAddress,
  updateAddress,
  updateFuncActionBar,
  onClickBack,
}) {
  const { signer, setSigner } = useContext(SignerClientContext);
  const [stage, setStage] = useState(STAGE_INIT);
  const [valueInputAddres, setValueInputAddres] = useState('');
  const [connectMethod, setConnectMethod] = useState<ConnectMethod | ''>('');
  const selectNetwork = 'cyber';
  const [validAddressAddedUser, setValidAddressAddedUser] = useState(true);

  const dispatch = useDispatch();

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
    switch (connectMethod) {
      case KEY_TYPE.keplr:
        connectKeplr();
        break;
      case KEY_TYPE.secrets:
        onClickToggleSecrets();
        break;
      case KEY_TYPE.wallet:
        setStage(STAGE_OPEN_MODAL);
        break;
      default:
        onClickAddAddressUser();
        break;
    }
  };

  const clearState = () => {
    setStage(STAGE_INIT);
    setValueInputAddres('');
    setConnectMethod('');
    setValidAddressAddedUser(true);
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
    updateAddress?.();
    updateFuncActionBar?.();
  };

  const connectKeplr = async () => {
    if (signer) {
      const { bech32Address, pubKey, name } = await signer.keplr.getKey(
        CHAIN_ID
      );
      const pk = toHex(pubKey);

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
      if (updateFuncActionBar) {
        updateFuncActionBar();
      }
    }
  };

  const connectKeplrFromMnemonic = async (name: string, mnemonic: string) => {
    const offlineSigner = await getOfflineSigner(mnemonic);
    setMnemonic(mnemonic);

    if (offlineSigner) {
      setSigner(offlineSigner);
      const [{ address, pubkey: pubKey }] = await offlineSigner.getAccounts();
      const pk = toHex(pubKey);

      const accounts: AccountValue = {
        pk,
        keys: 'keplr',
        path: HDPATH,
        name,
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
        onAdd={connectKeplrFromMnemonic}
        onCancel={() => {
          setStage(STAGE_INIT);
        }}
      />
    );
  }

  if (stage === STAGE_INIT) {
    return (
      <ConnectAddress
        selectMethodFunc={selectMethodFunc}
        selectMethod={connectMethod}
        selectNetwork={selectNetwork}
        connectAddress={connectAddress}
        signer={signer}
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

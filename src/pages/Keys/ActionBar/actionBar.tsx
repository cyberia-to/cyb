import { Pane } from '@cybercongress/gravity';
import { useEffect, useState } from 'react';
import { useDispatch, useSelector } from 'react-redux';
import { ActionBar as ActionBarContainer } from 'src/components';
import Button from 'src/components/btnGrd';
import { useSigningClient } from 'src/contexts/signerClient';
import imgRead from 'src/image/duplicate-outline.svg';
import imgKeplr from 'src/image/keplr-icon.svg';
import { deleteAddress } from 'src/redux/features/pocket';
import { removeSecret } from 'src/redux/reducers/scripting';
import { RootState } from 'src/redux/store';
import BroadcastChannelSender from 'src/services/backend/channels/BroadcastChannelSender';
import { KEY_LIST_TYPE, KEY_TYPE } from '../types';
import ActionBarConnect from './actionBarConnect';
import ActionBarKeplr from './actionBarKeplr';

const STAGE_INIT = 1;
const STAGE_CONNECT = 2;
const _STAGE_SEND_LEDGER = 3.1;
const STAGE_SEND_KEPLR = 4.1;
const STAGE_SEND_READ_ONLY = 5.1;

function ButtonImgText({ img, text = 'Send', ...props }) {
  return (
    <Button style={{ margin: '0 10px' }} {...props}>
      {text}{' '}
      <img
        style={{
          width: 20,
          height: 20,
          marginLeft: '5px',
          paddingTop: '2px',
        }}
        src={img}
        alt="img"
      />
    </Button>
  );
}

type Props = {
  selectCard: string;
  selectAccount: any;

  hoverCard?: string;
  updateAddress: () => void;
  keyType: string;

  selectedAddress?: string;

  defaultAccounts: {
    cyber: {
      keys: string;
    };
  } | null;
  defaultAccountsKeys: string | null;
};

function ActionBar({
  selectCard,
  selectAccount,
  hoverCard,
  keyType,
  selectedAddress,
  // global props
  updateAddress,
  defaultAccounts,
  defaultAccountsKeys,
}: Props) {
  const { signer: keplr } = useSigningClient();
  const [typeActionBar, setTypeActionBar] = useState('');
  const [stage, setStage] = useState(STAGE_INIT);
  const [makeActive, setMakeActive] = useState(false);
  const [connect, setConnect] = useState(false);

  const dispatch = useDispatch();
  const { accounts, defaultAccount } = useSelector((store: RootState) => store.pocket);

  const changeActionBar = (account) => {
    if (account !== null) {
      if (account.cyber) {
        const { keys } = account.cyber;
        setTypeActionBar(keys);
      } else {
        setTypeActionBar('noCyber');
      }
    }
  };

  useEffect(() => {
    if (stage === STAGE_INIT) {
      setMakeActive(false);
      setTypeActionBar('');
      switch (true) {
        case selectCard === 'tweet' || hoverCard === 'tweet':
          setTypeActionBar('tweet');
          break;

        case selectCard.indexOf('pubkey') !== -1 || hoverCard.indexOf('pubkey') !== -1:
          changeActionBar(selectAccount);
          break;

        default:
          setTypeActionBar('');
          break;
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectCard, hoverCard, selectAccount, changeActionBar, stage]);

  useEffect(() => {
    if (defaultAccountsKeys !== null && selectAccount !== null) {
      if (selectAccount.key && defaultAccountsKeys === selectAccount.key) {
        setMakeActive(false);
      } else {
        setMakeActive(true);
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectAccount, defaultAccountsKeys]);

  useEffect(() => {
    if (selectAccount !== null) {
      if (selectAccount.cyber && selectAccount.cosmos && selectAccount.eth) {
        setConnect(false);
      } else {
        setConnect(true);
      }
    }
  }, [selectAccount]);

  async function changeDefaultAccounts() {
    const accountName =
      accounts &&
      Object.entries(accounts).find((entry) => entry[1]?.cyber?.bech32 === selectedAddress)?.[0];

    if (accountName) {
      const broadcastChannel = new BroadcastChannelSender();
      broadcastChannel.postSetDefaultAccount(accountName);
    }
  }

  // const onClickDefaultAccountSend = () => {
  //   if (defaultAccounts !== null && defaultAccounts.cyber) {
  //     if (defaultAccounts.cyber.keys === 'keplr') {
  //       setStage(STAGE_SEND_KEPLR);
  //     }
  //     if (defaultAccounts.cyber.keys === 'ledger') {
  //       setStage(STAGE_SEND_LEDGER);
  //     }
  //     if (defaultAccounts.cyber.keys === 'read-only') {
  //       setStage(STAGE_SEND_READ_ONLY);
  //     }
  //   }
  // };

  const updateFuncActionBar = () => {
    setTypeActionBar('');
    setStage(STAGE_INIT);
    setMakeActive(false);
    setConnect(false);
  };

  const buttonConnect = (
    <Button style={{ margin: '0 10px' }} onClick={() => setStage(STAGE_CONNECT)}>
      add new key
    </Button>
  );

  const buttonActivate = (
    <Button style={{ margin: '0 10px' }} onClick={() => changeDefaultAccounts()}>
      Activate
    </Button>
  );

  const onDeleteClick = () => {
    if (!selectedAddress) {
      return;
    }

    if (keyType === KEY_LIST_TYPE.key) {
      dispatch(deleteAddress(selectedAddress));
    } else {
      dispatch(removeSecret({ key: selectedAddress }));
    }
    updateAddress();
  };

  if (selectedAddress) {
    return (
      <ActionBarContainer>
        <Pane display="flex">
          {defaultAccount.account?.cyber?.bech32 !== selectedAddress &&
            keyType === KEY_LIST_TYPE.key &&
            buttonActivate}

          <Button onClick={onDeleteClick}>Delete</Button>
        </Pane>
      </ActionBarContainer>
    );
  }

  if (stage === STAGE_CONNECT) {
    return (
      <ActionBarConnect
        updateAddress={updateAddress}
        updateFuncActionBar={updateFuncActionBar}
        selectAccount={selectAccount}
        onClickBack={() => setStage(STAGE_INIT)}
      />
    );
  }

  if (stage === STAGE_INIT) {
    if (typeActionBar === '') {
      return (
        <ActionBarContainer>
          <Pane display="flex">
            {buttonConnect}
            {/* {defaultAccounts !== null && defaultAccounts.cyber && (
              <Button
                style={{ margin: '0 10px' }}
                onClick={() => onClickDefaultAccountSend()}
              >
                Send
              </Button>
            )} */}
          </Pane>
        </ActionBarContainer>
      );
    }

    if (typeActionBar === 'noCyber') {
      return (
        <ActionBarContainer>
          <Pane>
            {connect && buttonConnect}
            {makeActive && buttonActivate}
          </Pane>
        </ActionBarContainer>
      );
    }
    if (typeActionBar === KEY_TYPE.keplr) {
      return (
        <ActionBarContainer>
          <Pane>
            {connect && buttonConnect}
            {keplr && <ButtonImgText img={imgKeplr} onClick={() => setStage(STAGE_SEND_KEPLR)} />}
            {makeActive && buttonActivate}
          </Pane>
        </ActionBarContainer>
      );
    }

    if (typeActionBar === KEY_TYPE.readOnly) {
      return (
        <ActionBarContainer>
          <Pane>
            {connect && buttonConnect}
            <ButtonImgText img={imgRead} onClick={() => setStage(STAGE_SEND_READ_ONLY)} />
            {makeActive && buttonActivate}
          </Pane>
        </ActionBarContainer>
      );
    }
  }

  if (stage === STAGE_SEND_KEPLR) {
    return (
      <ActionBarKeplr
        updateAddress={updateFuncActionBar}
        updateBalance={updateAddress}
        onClickBack={() => setStage(STAGE_INIT)}
      />
    );
  }

  return null;
}

export default ActionBar;

import { useState } from 'react';
import { useDispatch, useSelector } from 'react-redux';
import { Display, DisplayTitle } from 'src/components';
import ActionBar from 'src/pages/Keys/ActionBar/actionBar';
import { initPocket } from 'src/redux/features/pocket';
import { RootState } from 'src/redux/store';
import KeyItem from './KeyItem/KeyItem';
import KeyItemSecrets from './KeyItem/KeyItemSecrets';
import styles from './Keys.module.scss';
import { KEY_LIST_TYPE } from './types';

function Keys() {
  const { accounts } = useSelector((state: RootState) => state.pocket);
  const { secrets } = useSelector((state: RootState) => state.scripting.context);

  const [selectedKey, setSelectedKey] = useState<string | null>();
  const [keyType, setKeyType] = useState<string>(KEY_LIST_TYPE.key);

  const dispatch = useDispatch();

  function selectKey(keyType: string, address: string) {
    setKeyType(keyType);
    setSelectedKey(selectedKey === address ? null : address);
  }

  const bostromAccounts = accounts && Object.values(accounts).filter((account) => account?.cyber);

  return (
    <>
      <Display noPadding title={<DisplayTitle title="Keys" />}>
        <div className={styles.wrapper}>
          {bostromAccounts && bostromAccounts.length > 0 ? (
            bostromAccounts.map(({ cyber: account }) => {
              return (
                <KeyItem
                  key={account.bech32}
                  account={account}
                  selected={selectedKey === account.bech32}
                  selectKey={(addr) => selectKey(KEY_LIST_TYPE.key, addr)}
                />
              );
            })
          ) : (
            <p>
              you have no keys added yet <br />
              add your first key by connecting your wallet
            </p>
          )}
          {Object.keys(secrets).map((name) => {
            return (
              <KeyItemSecrets
                key={name}
                name={name}
                value={secrets[name]}
                selected={selectedKey === name}
                selectKey={(keyName) => selectKey(KEY_LIST_TYPE.secret, keyName)}
              />
            );
          })}
        </div>
      </Display>

      <ActionBar
        selectCard="pubkey"
        hoverCard="pubkey"
        keyType={keyType}
        selectAccount={null}
        selectedAddress={selectedKey}
        updateAddress={() => {
          dispatch(initPocket());
          setSelectedKey(null);
        }}
        defaultAccounts={null}
      />
    </>
  );
}

export default Keys;

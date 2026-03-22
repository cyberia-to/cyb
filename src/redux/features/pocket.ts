import { createSlice, PayloadAction } from '@reduxjs/toolkit';
import { Dispatch } from 'redux';
import { localStorageKeys } from 'src/constants/localStorageKeys';
import { Account, Accounts, AccountValue, DefaultAccount } from 'src/types/defaultAccount';
import { POCKET } from '../../utils/config';
import { removeEncryptedMnemonic } from '../../utils/utils';
import { RootState } from '../store';

type SliceState = {
  actionBar: {
    tweet: string;
  };
  defaultAccount: DefaultAccount;
  accounts: null | Accounts;
  isInitialized: boolean;
};

const initialState: SliceState = {
  actionBar: {
    tweet: POCKET.STAGE_TWEET_ACTION_BAR.TWEET, // stage for tweet ActionBar: 'addAvatar' 'follow' 'tweet'
  },
  isInitialized: false,
  defaultAccount: {
    name: null,
    account: null,
  },
  accounts: null,
};

const checkAddress = (obj, network, address) =>
  Object.keys(obj).some((k) => {
    if (obj[k][network]) {
      return obj[k][network].bech32 === address;
    }
  });

function saveToLocalStorage(state: SliceState) {
  const { defaultAccount, accounts } = state;

  defaultAccount &&
    localStorage.setItem(
      localStorageKeys.pocket.POCKET,
      JSON.stringify({
        [defaultAccount.name]: defaultAccount.account,
      })
    );
  accounts &&
    localStorage.setItem(localStorageKeys.pocket.POCKET_ACCOUNT, JSON.stringify(accounts));
}

const slice = createSlice({
  name: 'pocket',
  initialState,
  reducers: {
    setDefaultAccount: (
      state,
      { payload: { name, account } }: PayloadAction<{ name: string; account?: Account }>
    ) => {
      state.defaultAccount = {
        name,
        account: account || state.accounts?.[name] || null,
      };

      saveToLocalStorage(state);
    },
    setAccounts: (state, { payload }: PayloadAction<Accounts>) => {
      state.accounts = payload;

      saveToLocalStorage(state);
    },
    setInitialized: (state) => {
      state.isInitialized = true;
    },
    setStageTweetActionBar: (state, { payload }: PayloadAction<string>) => {
      state.actionBar.tweet = payload;
    },

    // bullshit
    deleteAddress: (state, { payload }: PayloadAction<string>) => {
      if (state.accounts) {
        Object.keys(state.accounts).forEach((accountKey) => {
          Object.keys(state.accounts[accountKey]).forEach((networkKey) => {
            if (state.accounts[accountKey][networkKey].bech32 === payload) {
              // Clean up encrypted mnemonic from localStorage if this was a wallet account
              if (state.accounts[accountKey][networkKey].keys === 'wallet') {
                removeEncryptedMnemonic(payload);
              }
              delete state.accounts[accountKey][networkKey];

              if (Object.keys(state.accounts[accountKey]).length === 0) {
                delete state.accounts[accountKey];
              }

              if (state.defaultAccount?.account?.cyber?.bech32 === payload) {
                const entries = Object.entries(state.accounts);

                const entryCyber = entries.find(([, value]) => value.cyber?.bech32);

                if (entryCyber) {
                  state.defaultAccount = {
                    name: entryCyber[0],
                    account: entryCyber[1],
                  };
                } else {
                  state.defaultAccount = {
                    name: null,
                    account: null,
                  };
                }
              }

              saveToLocalStorage(state);
            }
          });
        });
      }
    },
  },
});

export const selectCurrentAddress = (store: RootState) =>
  store.pocket.defaultAccount.account?.cyber?.bech32;

export const { setDefaultAccount, setAccounts, setStageTweetActionBar, deleteAddress } =
  slice.actions;

export default slice.reducer;

// refactor this
// Migrate legacy 'keplr' accounts to 'read-only' (one-time, idempotent)
function migrateKeplrAccounts() {
  try {
    const raw = localStorage.getItem(localStorageKeys.pocket.POCKET_ACCOUNT);
    if (!raw) return;

    let changed = false;
    const accounts: Accounts = JSON.parse(raw);

    Object.keys(accounts).forEach((name) => {
      Object.keys(accounts[name]).forEach((network) => {
        if (accounts[name][network].keys === 'keplr') {
          accounts[name][network].keys = 'read-only';
          changed = true;
        }
      });
    });

    if (changed) {
      localStorage.setItem(localStorageKeys.pocket.POCKET_ACCOUNT, JSON.stringify(accounts));

      // Also patch the default account pocket entry
      const pocketRaw = localStorage.getItem(localStorageKeys.pocket.POCKET);
      if (pocketRaw) {
        const pocket = JSON.parse(pocketRaw);
        Object.keys(pocket).forEach((name) => {
          if (pocket[name]) {
            Object.keys(pocket[name]).forEach((network) => {
              if (pocket[name][network]?.keys === 'keplr') {
                pocket[name][network].keys = 'read-only';
              }
            });
          }
        });
        localStorage.setItem(localStorageKeys.pocket.POCKET, JSON.stringify(pocket));
      }

      // Flag for one-time adviser notification
      if (!localStorage.getItem('cyb:keplr-migrated')) {
        localStorage.setItem('cyb:keplr-migrated', '1');
      }
    }
  } catch (e) {
    console.warn('keplr account migration skipped due to corrupt localStorage:', e);
  }
}

export const initPocket = () => (dispatch: Dispatch) => {
  // Migrate keplr accounts before loading
  migrateKeplrAccounts();

  let defaultAccounts = null;
  let defaultAccountsKeys = null;
  let accountsTemp: Accounts | null = null;

  const localStoragePocketAccount = localStorage.getItem(localStorageKeys.pocket.POCKET_ACCOUNT);
  const localStoragePocket = localStorage.getItem(localStorageKeys.pocket.POCKET);
  if (localStoragePocket !== null) {
    const localStoragePocketData = JSON.parse(localStoragePocket);
    const keyPocket = Object.keys(localStoragePocketData)[0];
    const accountPocket = Object.values(localStoragePocketData)[0];
    defaultAccounts = accountPocket;
    defaultAccountsKeys = keyPocket;
  }
  if (localStoragePocketAccount !== null) {
    const localStoragePocketAccountData = JSON.parse(localStoragePocketAccount);
    if (localStoragePocket === null) {
      const keys0 = Object.keys(localStoragePocketAccountData)[0];
      localStorage.setItem(
        localStorageKeys.pocket.POCKET,
        JSON.stringify({ [keys0]: localStoragePocketAccountData[keys0] })
      );
      defaultAccounts = localStoragePocketAccountData[keys0];
      defaultAccountsKeys = keys0;
    } else if (defaultAccountsKeys !== null) {
      accountsTemp = {
        [defaultAccountsKeys]: localStoragePocketAccountData[defaultAccountsKeys] || undefined,
        ...localStoragePocketAccountData,
      };
    }
  } else {
    localStorage.removeItem(localStorageKeys.pocket.POCKET);
    localStorage.removeItem(localStorageKeys.pocket.POCKET_ACCOUNT);
  }

  defaultAccountsKeys &&
    defaultAccounts &&
    dispatch(
      setDefaultAccount({
        name: defaultAccountsKeys,
        account: defaultAccounts,
      })
    );

  accountsTemp &&
    Object.keys(accountsTemp).forEach((key) => {
      if (!accountsTemp[key] || Object.keys(accountsTemp[key]).length === 0) {
        delete accountsTemp[key];
      }
    });

  accountsTemp && dispatch(setAccounts(accountsTemp));

  dispatch(slice.actions.setInitialized());
};

const defaultNameAccount = () => {
  let key = 'Account 1';
  let count = 1;

  const localStorageCount = localStorage.getItem('count');

  if (localStorageCount !== null) {
    const dataCount = JSON.parse(localStorageCount);
    count = parseFloat(dataCount);
    key = `Account ${count}`;
  }

  localStorage.setItem('count', JSON.stringify(count + 1));

  return key;
};

export const addAddressPocket = (accounts: AccountValue) => (dispatch: Dispatch) => {
  const key = accounts.name || defaultNameAccount();

  let dataPocketAccount = null;
  let valueObj = {};
  let pocketAccount: Accounts = {};

  const localStorageStory = localStorage.getItem(localStorageKeys.pocket.POCKET_ACCOUNT);

  if (localStorageStory !== null) {
    dataPocketAccount = JSON.parse(localStorageStory);
    valueObj = Object.values(dataPocketAccount);
  }

  const isAdded = !checkAddress(valueObj, 'cyber', accounts.bech32);

  if (!isAdded) {
    return;
  }

  const cyberAccounts: Account = {
    cyber: accounts,
  };

  if (localStorageStory !== null) {
    pocketAccount = { [key]: cyberAccounts, ...dataPocketAccount };
  } else {
    pocketAccount = { [key]: cyberAccounts };
  }

  if (Object.keys(pocketAccount).length > 0) {
    dispatch(setAccounts(pocketAccount));
    if (accounts.keys !== 'read-only') {
      dispatch(setDefaultAccount({ name: key, account: cyberAccounts }));
    }
  }
};

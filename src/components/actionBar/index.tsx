import React, { useEffect, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { CHAIN_ID } from 'src/constants/config';
import { useSigningClient } from 'src/contexts/signerClient';
import { useAdviser } from 'src/features/adviser/context';
import { AdviserColors } from 'src/features/adviser/Adviser/Adviser';
import usePassportByAddress from 'src/features/passport/hooks/usePassportByAddress';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppSelector } from 'src/redux/hooks';
import { routes } from 'src/routes';
import { Networks } from 'src/types/networks';
import { $TsFixMeFunc } from 'src/types/tsfix';
import { trimString } from 'src/utils/utils';
import Button from '../btnGrd';
import ButtonIcon from '../buttons/ButtonIcon';
import { Input } from '../Input';
import styles from './styles.module.scss';

const back = require('../../image/arrow-left-img.svg');

function ActionBarContainer({ children }: { children: React.ReactNode }) {
  return (
    <div className={styles.ActionBarContainer}>
      <div className={styles.ActionBarContainerContent}>{children}</div>
    </div>
  );
}

type Props = {
  children?: React.ReactNode;
  onClickBack?: $TsFixMeFunc;
  text?: string | React.ReactNode;
  button?: {
    text: string | React.ReactNode;
    onClick?: () => void;
    link?: string;
    disabled?: boolean;
    pending?: boolean;
  };
};

function ActionBar({ children, text, onClickBack, button }: Props) {
  const { signerReady } = useSigningClient();
  const location = useLocation();

  const defaultAccount = useAppSelector((store) => store.pocket.defaultAccount);
  const commander = useAppSelector((store) => store.commander);

  const address = useAppSelector(selectCurrentAddress);
  const { passport } = usePassportByAddress(address);
  const noAccount = !defaultAccount.account;
  const noPassport = CHAIN_ID === Networks.BOSTROM && !passport;

  // FIXME: refactor
  const exception =
    (!location.pathname.includes('/keys') &&
      !location.pathname.includes('/drive') &&
      // !location.pathname.includes('/oracle') &&
      location.pathname !== '/') ||
    location.pathname === '/oracle/learn';
  // TODO: not show while loading passport

  // on mobile, commander is in MobileMenuBar — don't override actionBar
  if (commander.isFocused && window.innerWidth > 768) {
    return (
      <ActionBarContainer>
        <Button link={routes.search.getLink(commander.value)} disabled={!commander.value.length}>
          Ask
        </Button>
      </ActionBarContainer>
    );
  }

  if (
    noAccount &&
    // maybe change to props
    exception &&
    !location.pathname.includes(routes.gift.path) &&
    !location.pathname.includes('/brain') && // both full and robot
    !location.pathname.includes('/mining') &&
    !location.pathname.includes('/network/bostrom/tx')
  ) {
    return (
      <ActionBarContainer>
        <Button link={routes.keys.path}>Connect</Button>
      </ActionBarContainer>
    );
  }

  if (
    !signerReady &&
    exception &&
    !location.pathname.includes(routes.gift.path) &&
    !location.pathname.includes('/brain') // both full and robot
  ) {
    if (process.env.IS_TAURI) {
      return (
        <ActionBarContainer>
          <Button link={routes.keys.path}>Connect wallet</Button>
        </ActionBarContainer>
      );
    }

    const keys = defaultAccount.account?.cyber.keys;
    const isWallet = keys === 'wallet' || keys === 'private-key';
    const isLedger = keys === 'ledger';

    if (isWallet) {
      return <UnlockWalletBar />;
    }

    if (isLedger) {
      return <ConnectLedgerBar />;
    }

    const activeAddress =
      defaultAccount.account?.cyber.name || trimString(defaultAccount.account?.cyber.bech32, 10, 4);

    return (
      <ActionBarContainer>
        <span>
          choose {defaultAccount.account?.cyber.name ? 'account' : 'address'}{' '}
          <span className={styles.chooseAccount}>{activeAddress}</span> in your wallet
        </span>
      </ActionBarContainer>
    );
  }

  const content = text || children;

  const contentPortal = (
    <ActionBarContainer>
      {/* <Telegram /> */}

      {onClickBack && (
        <div className={styles.backButton}>
          <ButtonIcon
            style={{ padding: 0 }}
            img={back}
            onClick={onClickBack}
            text="previous step"
          />
        </div>
      )}

      {content && <div className={styles.ActionBarContentText}>{content}</div>}

      {button?.text && (
        <Button
          disabled={button.disabled}
          pending={button.pending}
          link={button.link}
          onClick={button.onClick}
        >
          {button.text}
        </Button>
      )}
      {/* <GitHub /> */}
    </ActionBarContainer>
  );

  // const portalEl = document.getElementById('portalActionBar');

  // return portalEl ? createPortal(contentPortal, portalEl) : contentPortal;

  return contentPortal;
}

function UnlockWalletBar() {
  const { unlockWallet } = useSigningClient();
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const handleUnlock = async () => {
    setError('');
    setLoading(true);
    try {
      await unlockWallet(password);
      setPassword('');
    } catch (err) {
      setError('Wrong password');
      setPassword('');
    } finally {
      setLoading(false);
    }
  };

  return (
    <ActionBarContainer>
      <span style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
        <Input
          width="200px"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          placeholder="enter password to unlock"
          type="password"
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck={false}
          onKeyDown={(e) => e.key === 'Enter' && handleUnlock()}
          autoFocus
        />
        <Button onClick={handleUnlock} disabled={!password || loading}>
          {loading ? 'Unlocking...' : 'Unlock'}
        </Button>
        {error && <span style={{ color: '#ff4d4d', fontSize: '14px' }}>{error}</span>}
      </span>
    </ActionBarContainer>
  );
}

function ConnectLedgerBar() {
  const { reconnectLedger } = useSigningClient();
  const { setAdviser } = useAdviser();
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    setAdviser(
      'Ledger is not connected. Wake up your device and open the Cosmos app',
      AdviserColors.yellow
    );
  }, [setAdviser]);

  const handleConnect = async () => {
    setError('');
    setLoading(true);
    try {
      await reconnectLedger();
    } catch (err: any) {
      setError(err?.message || 'Connection failed');
      setAdviser(
        `Could not connect to Ledger: ${err?.message || 'unknown error'}`,
        AdviserColors.red
      );
    } finally {
      setLoading(false);
    }
  };

  return (
    <ActionBarContainer>
      <Button onClick={handleConnect} disabled={loading}>
        {loading ? 'Connecting...' : 'Connect Ledger'}
      </Button>
    </ActionBarContainer>
  );
}

export default ActionBar;

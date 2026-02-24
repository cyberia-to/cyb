import React from 'react';
import { useLocation } from 'react-router-dom';
import { CHAIN_ID } from 'src/constants/config';
import { useDevice } from 'src/contexts/device';
import { useSigningClient } from 'src/contexts/signerClient';
import usePassportByAddress from 'src/features/passport/hooks/usePassportByAddress';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppSelector } from 'src/redux/hooks';
import { routes } from 'src/routes';
import { Networks } from 'src/types/networks';
import { $TsFixMeFunc } from 'src/types/tsfix';
import { trimString } from 'src/utils/utils';
import Button from '../btnGrd';
import ButtonIcon from '../buttons/ButtonIcon';
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

  const { defaultAccount, commander } = useAppSelector((store) => {
    return {
      defaultAccount: store.pocket.defaultAccount,
      commander: store.commander,
    };
  });

  const address = useAppSelector(selectCurrentAddress);
  const { passport } = usePassportByAddress(address);
  const { isMobile } = useDevice();

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

  if (commander.isFocused) {
    return (
      <ActionBarContainer>
        <Button link={routes.search.getLink(commander.value)} disabled={!commander.value.length}>
          Ask
        </Button>
      </ActionBarContainer>
    );
  }

  if (
    (noAccount || noPassport) &&
    // maybe change to props
    exception &&
    !location.pathname.includes(routes.gift.path) &&
    !location.pathname.includes('/brain') && // both full and robot
    !location.pathname.includes('/mining') &&
    (!isMobile || process.env.IS_TAURI)
  ) {
    return (
      <ActionBarContainer>
        {noAccount && <Button link={routes.keys.path}>Connect</Button>}

        {noPassport && location.pathname !== routes.citizenship.path && (
          <Button link={routes.portal.path}>Get citizenship</Button>
        )}
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

    const activeAddress =
      defaultAccount.account?.cyber.name || trimString(defaultAccount.account?.cyber.bech32, 10, 4);

    return (
      <ActionBarContainer>
        <span>
          choose {defaultAccount.account?.cyber.name ? 'account' : 'address'}{' '}
          <span className={styles.chooseAccount}>{activeAddress}</span> in keplr
        </span>
      </ActionBarContainer>
    );
  }

  const content = text || children;

  const contentPortal = (
    <ActionBarContainer>
      {/* <Telegram /> */}

      {onClickBack && (
        <ButtonIcon
          styleContainer={{ position: 'absolute', left: '0' }}
          style={{ padding: 0 }}
          img={back}
          onClick={onClickBack}
          text="previous step"
        />
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

export default ActionBar;

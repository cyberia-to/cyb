import cx from 'classnames';
import React, { useMemo, useRef } from 'react';
import { usePopperTooltip } from 'react-popper-tooltip';
import { Link, useLocation } from 'react-router-dom';
import { Transition } from 'react-transition-group';
import Pill from 'src/components/Pill/Pill';
import { useBackend } from 'src/contexts/backend/backend';
import { useDevice } from 'src/contexts/device';
import { useSigningClient } from 'src/contexts/signerClient';
import usePassportByAddress from 'src/features/passport/hooks/usePassportByAddress';
import useIsOnline from 'src/hooks/useIsOnline';
import useOnClickOutside from 'src/hooks/useOnClickOutside';
import { useAppSelector } from 'src/redux/hooks';
import { routes } from 'src/routes';
import BroadcastChannelSender from 'src/services/backend/channels/BroadcastChannelSender';
import useMediaQuery from '../../../../hooks/useMediaQuery';
import robot from '../../../../image/temple/robot.png';
import { AvataImgIpfs } from '../../../portal/components/avataIpfs';
import Karma from '../../Karma/Karma';
import networkStyles from '../CurrentApp/CurrentApp.module.scss';
import styles from './SwitchAccount.module.scss';

// should be refactored
function AccountItem({
  data,
  onClickSetActive,
  setControlledVisible,
  name: accountName,
  image,
  link,
}) {
  const address = data?.cyber?.bech32;
  const location = useLocation();

  const { passport } = usePassportByAddress(address);

  const name = passport?.extension?.nickname || data?.cyber?.name || accountName;

  const nickname = passport?.extension?.nickname;

  function handleClick() {
    onClickSetActive?.();
    setControlledVisible(false);
  }

  const robotPath = nickname ? routes.robotPassport.getLink(nickname) : routes.robot.path;

  const linkToRobot = location.pathname.includes('@') ? robotPath : undefined;

  return (
    <Link
      to={link || linkToRobot}
      onClick={handleClick}
      className={cx(styles.containerSwichAccount, networkStyles.btnContainerText)}
    >
      <div className={cx(networkStyles.containerInfoSwitch, styles.content)}>
        {name && (
          <span
            className={cx(networkStyles.btnContainerText, {
              [styles.noAccount]: !address,
            })}
          >
            {name}
          </span>
        )}
        {address && <Karma address={address} />}
      </div>
      <div className={cx(styles.containerAvatarConnect)}>
        <div className={styles.containerAvatarConnectTrue}>
          {image ? (
            <img className={styles.image} src={image} alt={name} />
          ) : (
            <AvataImgIpfs
              style={{ position: 'relative' }}
              cidAvatar={passport?.extension?.avatar}
              addressCyber={address}
            />
          )}
        </div>
      </div>
    </Link>
  );
}

function SwitchAccount() {
  const { signerReady } = useSigningClient();
  const { isIpfsInitialized } = useBackend();
  const isOnline = useIsOnline();
  const mediaQuery = useMediaQuery('(min-width: 768px)');
  const { isMobile } = useDevice();

  const [controlledVisible, setControlledVisible] = React.useState(false);

  const { defaultAccount, accounts } = useAppSelector((state) => state.pocket);

  const useGetAddress = defaultAccount?.account?.cyber?.bech32 || null;

  const { passport } = usePassportByAddress(useGetAddress);

  const { getTooltipProps, setTooltipRef, visible } = usePopperTooltip({
    trigger: 'click',
    closeOnTriggerHidden: true,
    visible: controlledVisible,
    onVisibleChange: setControlledVisible,
    placement: 'bottom',
  });

  const containerRef = useRef<HTMLDivElement>(null);

  useOnClickOutside(containerRef, () => {
    setControlledVisible(false);
  });

  const useGetCidAvatar = passport?.extension.avatar;
  const useGetName = passport?.extension.nickname || defaultAccount?.name;
  const isReadOnly = defaultAccount.account?.cyber.keys === 'read-only';

  const onClickChangeActiveAcc = async (key: string) => {
    const broadcastChannel = new BroadcastChannelSender();
    broadcastChannel.postSetDefaultAccount(key);
    setControlledVisible(false);
  };

  const renderItem = useMemo(() => {
    if (!accounts) {
      return null;
    }

    return Object.keys(accounts)
      .filter((item) => defaultAccount.name !== item && accounts[item].cyber)
      .map((key) => {
        return (
          <AccountItem
            key={key}
            data={accounts[key]}
            onClickSetActive={() => onClickChangeActiveAcc(key)}
            setControlledVisible={setControlledVisible}
            name={key}
          />
        );
      });

    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [accounts, defaultAccount, onClickChangeActiveAcc]);

  // const multipleAccounts = renderItem && Object.keys(renderItem).length > 0;

  return (
    <div style={{ position: 'relative', fontSize: '20px' }} ref={containerRef}>
      <div className={styles.containerSwichAccount}>
        {(!useGetAddress || !mediaQuery) && (
          // eslint-disable-next-line react/jsx-no-useless-fragment
          <>
            {/* FIXME: because of styles */}
            {isMobile ? (
              <div />
            ) : (
              <Link className={networkStyles.btnContainerText} to={routes.settings.path}>
                {mediaQuery ? 'Settings' : '⚙️'}
              </Link>
            )}
          </>
        )}
        {mediaQuery && useGetAddress && (
          <div
            className={cx(
              networkStyles.containerInfoSwitch,
              isReadOnly || !signerReady ? networkStyles.containerInfoSwitchGapUnSet : undefined
            )}
          >
            {useGetName && (
              <button
                onClick={() => setControlledVisible((item) => !item)}
                className={networkStyles.btnContainerText}
                type="button"
                style={{ fontSize: '20px' }}
              >
                {useGetName}
                {/* {multipleAccounts && '>'} */}
              </button>
            )}
            {isReadOnly && <Pill color="yellow" text="read only" />}
            {!isReadOnly && !signerReady && isOnline && <Pill color="red" text="switch keplr" />}
            {!isOnline && <Pill color="red" text="offline" />}
            <Karma address={useGetAddress} />
          </div>
        )}
        <Link
          to={
            passport ? routes.robotPassport.getLink(passport.extension.nickname) : routes.robot.path
          }
          className={styles.content}
          // onClick={() => setControlledVisible(!controlledVisible)}
        >
          <div
            className={cx(styles.containerAvatarConnect, {
              [styles.containerAvatarConnectFalse]: !isIpfsInitialized || !isOnline,
              [styles.containerAvatarConnectTrueGreen]: isIpfsInitialized && isOnline,
            })}
          >
            <AvataImgIpfs
              style={{
                position: 'relative',
                objectFit: !useGetCidAvatar && 'contain',
              }}
              cidAvatar={useGetCidAvatar}
              addressCyber={useGetAddress}
              img={robot}
            />
          </div>
        </Link>
      </div>
      <Transition in={visible} timeout={300}>
        {(state) => {
          return (
            <div
              ref={setTooltipRef}
              {...getTooltipProps({
                className: styles.tooltipContainerRight,
              })}
            >
              <div
                className={cx(styles.containerSwichAccountList, [
                  styles[`containerSwichAccountList_${state}`],
                ])}
              >
                {renderItem}

                <AccountItem
                  name="supersigma"
                  link={routes.sigma.path}
                  setControlledVisible={setControlledVisible}
                  image={require('../../../../image/sigma.png')}
                />
                <AccountItem
                  name="settings"
                  setControlledVisible={setControlledVisible}
                  link={routes.settings.path}
                  image={require('./keys.png')}
                />
              </div>
            </div>
          );
        }}
      </Transition>
    </div>
  );
}

export default SwitchAccount;

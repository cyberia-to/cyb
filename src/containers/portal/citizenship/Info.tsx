/* eslint-disable no-await-in-loop */
/* eslint-disable no-restricted-syntax */
/* eslint-disable jsx-a11y/no-static-element-interactions */
/* eslint-disable jsx-a11y/click-events-have-key-events */
import { useMemo } from 'react';
import { Link } from 'react-router-dom';
import { LinkWindow } from '../../../components';
import { formatNumber } from '../../../utils/search/utils';
import { BOOT_ICON } from '../utils';
import { steps } from './utils';

const {
  STEP_INIT,
  STEP_NICKNAME_CHOSE,
  STEP_NICKNAME_APROVE,
  STEP_NICKNAME_INVALID,
  STEP_RULES,
  STEP_AVATAR_UPLOAD,
  STEP_WALLET_INIT,
  STEP_WALLET_INSTALLED,
  STEP_WALLET_CHECK,
  STEP_WALLET_SETUP,
  STEP_WALLET_CONNECT,
  STEP_CHECK_ADDRESS,
  STEP_REGISTER,
  STEP_DONE,
  STEP_CHECK_GIFT,
  STEP_CHECK_ADDRESS_CHECK_FNC,
  STEP_ACTIVE_ADD,
} = steps;

// generated, check
type Props = {
  stepCurrent: (typeof steps)[keyof typeof steps];
  nickname: string;
  valuePriceNickname: number;
  registerDisabled: boolean;
  setStep: (step: (typeof steps)[keyof typeof steps]) => void;
  counCitizenshipst: number;
  mobile: boolean;
};

function Info({
  stepCurrent,
  nickname,
  valuePriceNickname,
  registerDisabled,
  setStep,
  counCitizenshipst,
  mobile,
}: Props) {
  const useContent = useMemo(() => {
    let content;

    switch (stepCurrent) {
      case STEP_INIT:
        content = (
          <div
            style={{
              textAlign: 'center',
              padding: '10px 50px 0px 50px',
              gap: 20,
              display: 'grid',
            }}
          >
            <div>My name is Cyb.</div>
            <div>
              I have helped{' '}
              <span style={{ color: '#36d6ae' }}>{formatNumber(counCitizenshipst)}</span> beings
              receive Moon Citizenship.
            </div>
            <div>I can also assist you in 7 simple steps.</div>
            {mobile ? (
              <div>But only desktop :( </div>
            ) : (
              <div>If you work hard, you may get a gift!</div>
            )}
          </div>
        );
        break;

      case STEP_NICKNAME_CHOSE:
        content = (
          <span>
            Choose your nickname. You will own it as an NFT.
            <br />
            8+ symbols are free
          </span>
        );
        break;

      case STEP_NICKNAME_INVALID:
        content = (
          <span>
            Nickname is unavailable. <br />
            Choose another nickname
          </span>
        );
        break;

      case STEP_NICKNAME_APROVE:
        content = (
          <span>
            Nickname is available{' '}
            {valuePriceNickname && valuePriceNickname !== null && (
              <>
                for
                {` ${formatNumber(valuePriceNickname.amountPrice)} ${BOOT_ICON}`}
                <br /> 8+ symbols are freexs
              </>
            )}
          </span>
        );
        break;

      case STEP_RULES:
        content = <span>Dear {nickname}, sign the Moon Code before setting up your account.</span>;
        break;

      case STEP_AVATAR_UPLOAD:
        content = <span>Upload a gif or picture. You will also own this as an NFT.</span>;
        break;

      case STEP_WALLET_CHECK:
      case STEP_WALLET_INIT:
      case STEP_WALLET_INSTALLED:
        content = (
          <span>
            You need a wallet to use cyb. <br />
            Import your seed phrase or connect a Ledger.
          </span>
        );
        break;

      case STEP_WALLET_SETUP:
        content = (
          <span>
            Set up your wallet. <br />
            You will then have addresses in the Cyber ecosystem.
          </span>
        );
        break;

      case STEP_WALLET_CONNECT:
        content = (
          <span>
            Connect your wallet. <br /> One click left
          </span>
        );
        break;

      case STEP_ACTIVE_ADD:
      case STEP_CHECK_ADDRESS:
        content = (
          <span>
            Your passport can be registered <br /> after address activation.
          </span>
        );
        break;

      case STEP_CHECK_ADDRESS_CHECK_FNC:
        content = <span>Verification takes time. After that, you can register your passport.</span>;
        break;

      case STEP_REGISTER:
        if (!registerDisabled) {
          content = (
            <span>
              you need{' '}
              {valuePriceNickname &&
                valuePriceNickname !== null &&
                `${formatNumber(valuePriceNickname.amountPrice)} ${BOOT_ICON}`}{' '}
              for register your nickname, <br /> you can{' '}
              <span
                style={{ color: '#36d6ae', cursor: 'pointer' }}
                onClick={() => setStep(STEP_NICKNAME_CHOSE)}
              >
                change
              </span>{' '}
              nickname or <Link to="/teleport">buy {BOOT_ICON}</Link>, 8+ symbols are free
            </span>
          );
        } else {
          content = <span>Register passport, then check for a gift</span>;
        }
        break;

      case STEP_DONE:
      case STEP_CHECK_GIFT:
        content = (
          <span>
            Congratulations, {nickname} - <br /> you are a citizen of the Moon. Try your luck and
            check for a gift
          </span>
        );
        break;

      default:
        content = null;
        break;
    }

    return content;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    stepCurrent,
    counCitizenshipst,
    mobile,
    nickname,
    registerDisabled,
    setStep,
    valuePriceNickname,
  ]);

  return useContent;
}

export default Info;

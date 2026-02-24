import { Link } from 'react-router-dom';
import { routes } from 'src/routes';
import { formatNumber } from 'src/utils/utils';
import { BOOT_ICON } from '../utils';
import STEP_INFO from './utils';

const {
  STATE_INIT,
  STATE_INIT_NULL,
  STATE_INIT_PROVE,
  STATE_INIT_CLAIM,
  STATE_INIT_RELEASE,
  STATE_PROVE,
  STATE_PROVE_CONNECT,
  STATE_PROVE_SIGN_MM,
  STATE_PROVE_SIGN_KEPLR,
  STATE_PROVE_SEND_SIGN,
  STATE_PROVE_CHANGE_ACCOUNT,
  STATE_PROVE_YOU_ADDED_ADDR,
  STATE_CLAIME_TO_PROVE,
  STATE_GIFT_NULL_ALL,
  STATE_CLAIME,
  STATE_CLAIME_ALL,
  STATE_RELEASE,
  STATE_CLAIM_IN_PROCESS,
  STATE_PROVE_IN_PROCESS,
  STATE_RELEASE_INIT,
  STATE_RELEASE_ALL,
  STATE_RELEASE_NULL,
  STATE_RELEASE_IN_PROCESS,
} = STEP_INFO;

function Info({ stepCurrent, useReleasedStage, nextRelease }) {
  let content;

  switch (stepCurrent) {
    case STATE_INIT_NULL:
      content = (
        <span>
          Check gift & basic information. <br />
          Hurry up! Get your citizenship to be able to claim.
        </span>
      );
      break;

    case STATE_INIT:
    case STATE_INIT_PROVE:
      content = (
        <span>
          Check gift & basic information. <br />
          Hurry up! Prove an address to claim the gift.
        </span>
      );
      break;

    case STATE_INIT_CLAIM:
      content = (
        <span>
          You have unclaimed gifts - <br />
          go to claim
        </span>
      );
      break;

    case STATE_INIT_RELEASE:
      content = (
        <span>
          You have claimed all gifts. <br />
          Go to release or prove another address.
        </span>
      );
      break;

    case STATE_PROVE:
      content = (
        <span>
          Prove ethereum, cosmos, osmosis, <br /> terra signatures by selecting signer to check for
          the gift
        </span>
      );
      break;

    case STATE_PROVE_CONNECT:
      content = <span>Select signer</span>;
      break;

    case STATE_PROVE_SIGN_MM:
      content = <span>Sign message in metamask</span>;
      break;

    case STATE_PROVE_SIGN_KEPLR:
      content = <span>sign message in keplr</span>;
      break;

    case STATE_PROVE_SEND_SIGN:
      content = <span>send your signature</span>;
      break;

    case STATE_PROVE_CHANGE_ACCOUNT:
      content = <span> you need change account in keplr</span>;
      break;

    case STATE_PROVE_YOU_ADDED_ADDR:
      content = <span>you have already added this address. Сhoose another address</span>;
      break;

    case STATE_CLAIME_TO_PROVE:
      content = (
        <span>
          You did not work hard to get gift. No wories ! You have a citizenship, just go and buy
          BOOT
        </span>
      );
      break;

    case STATE_GIFT_NULL_ALL:
      content = <span>You have nothing to claim. Prove another address for the gift.</span>;
      break;

    case STATE_CLAIME:
      content = (
        <span>
          You have unclaimed gifts. <br />
          Claim now, or prove another address.
        </span>
      );
      break;

    case STATE_CLAIME_ALL:
      content = (
        <span>
          Chose bostrom address <br />
          to claim all gifts, or claim one by one
        </span>
      );
      break;

    case STATE_RELEASE:
      content = (
        <span>
          You claimed all gifts. <br />
          Go to release or prove another address.
        </span>
      );
      break;

    case STATE_RELEASE_INIT:
      content = (
        <span>
          release {formatNumber(useReleasedStage.availableRelease) || ''} {BOOT_ICON} right now!{' '}
          <br />
        </span>
      );
      break;

    case STATE_RELEASE_ALL:
      content = (
        <span>
          Next release will be available in {nextRelease} new addresses. <br />
          <Link to={routes.sphere.path}>Hire hero</Link> and get H token for free
          <br />
          invite your friends to release faster
        </span>
      );
      break;

    case STATE_RELEASE_NULL:
      content = (
        <span>
          You have nothing to release. <br />
          Prove another address <br />
          or claim address with the gift.
        </span>
      );
      break;

    case STATE_PROVE_IN_PROCESS:
      content = <span>prove address take time</span>;
      break;

    case STATE_CLAIM_IN_PROCESS:
      content = <span>claim take time</span>;
      break;

    case STATE_RELEASE_IN_PROCESS:
      content = <span>release take time</span>;
      break;

    default:
      content = null;
      break;
  }

  return content;
}

export default Info;

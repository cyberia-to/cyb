import { Link } from 'react-router-dom';
import { routes } from 'src/routes';
import { formatNumber } from 'src/utils/utils';
import { BOOT_ICON } from '../utils';
import { DataReleaseStatus } from './type';
import STEP_INFO from './utils';

const { STATE_READY_TO_RELEASE, STATE_NEXT_UNFREEZE, STATE_PROVE_ADDRESS } = STEP_INFO;

function Info({
  useReleasedStage,
  stepCurrent,
  nextRelease,
}: {
  stepCurrent: number;
  useReleasedStage: DataReleaseStatus;
  nextRelease: number;
}) {
  let content;

  switch (stepCurrent) {
    case STATE_READY_TO_RELEASE:
      content = (
        <span>
          release {formatNumber(useReleasedStage.availableRelease) || ''} {BOOT_ICON} right now!{' '}
          <br />
        </span>
      );
      break;

    case STATE_NEXT_UNFREEZE:
      content = (
        <span>
          Next release will be available in {nextRelease} new addresses. <br />
          <Link to={routes.sphere.path}>Hire hero</Link> and get H token for free
          <br />
          invite your friends to release faster
        </span>
      );
      break;

    case STATE_PROVE_ADDRESS:
      content = (
        <span>
          You have nothing to release. <br />
          Prove another address <br />
          or claim address with the gift.
        </span>
      );
      break;

    default:
      content = null;
      break;
  }

  return content;
}

export default Info;

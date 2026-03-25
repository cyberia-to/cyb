import { LinkWindow } from 'src/components/link/link';
import twitterIcon from './twitter-x-icon.svg';

function Twitter() {
  return (
    <LinkWindow to="https://twitter.com/cyber_devs">
      <img src={twitterIcon} alt="Twitter" />
      <span>x (twitter)</span>
    </LinkWindow>
  );
}

export default Twitter;

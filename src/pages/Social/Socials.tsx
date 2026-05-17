import { DisplayTitle, LinkWindow } from 'src/components';
import Discord from './Discord/Discord';
import { GitHub } from './GitHub/GitHub';
import { HUB_LINK } from './Social';
import styles from './Social.module.scss';
import Telegram from './Telegram/Telegram';
import Twitter from './Twitter/Twitter';

export default function Socials() {
  return (
    <div className={styles.wrapper}>
      <div className={styles.main}>
        <Discord />
        <Twitter />
        <Telegram />
      </div>

      <DisplayTitle inDisplay title="code" />

      <div className={styles.code}>
        <GitHub />
      </div>
      <br />
      <DisplayTitle inDisplay title="More links" />

      <LinkWindow className={styles.hubLinks} to={HUB_LINK}>
        <div>👾</div>
        <span>Hub links</span>
      </LinkWindow>
      <br />
      <DisplayTitle inDisplay title="websites" />

      <LinkWindow className={styles.hubLinks} to="https://bostrom.network/">
        <div>🌐</div>
        <span>bostrom.network</span>
      </LinkWindow>

      <br />
      <DisplayTitle inDisplay title="other" />
      <a
        href="mailto:bostromboot@gmail.com"
        target="_blank"
        rel="noreferrer noopener"
        className={styles.email}
      >
        bostromboot@gmail.com
      </a>
    </div>
  );
}

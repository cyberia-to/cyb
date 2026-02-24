import { LinkWindow } from '../../../components/link/link';
import styles from './Telegram.module.scss';
import telegram from './telegram-icon.svg';

function Telegram() {
  return (
    <div className={styles.wrapper}>
      <LinkWindow to="https://t.me/CyberGlobalHub">
        <img alt="telegram" style={{ width: 30, height: 30 }} src={telegram} />

        <span>t/eng</span>
      </LinkWindow>

      <LinkWindow to="https://t.me/bostrom_news">
        <img alt="telegram" style={{ width: 30, height: 30 }} src={telegram} />

        <span>t/news</span>
      </LinkWindow>

      <LinkWindow to="https://t.me/fameofcyber">
        <img alt="telegram" style={{ width: 30, height: 30 }} src={telegram} />

        <span>t/validators</span>
      </LinkWindow>
    </div>
  );
}

export default Telegram;

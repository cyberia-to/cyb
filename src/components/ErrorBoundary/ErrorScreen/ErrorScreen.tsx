import BtnGrd from '../../btnGrd';
import styles from './ErrorScreen.module.scss';

import robot from '../../../image/robot.svg';
import { LinkWindow } from 'src/components/link/link';
import { HUB_LINK } from 'src/pages/Social/Social';

function ErrorScreen({ error }: { error: Error }) {
  return (
    <div className={styles.wrapper}>
      <img src={robot} alt="Robot" />

      <p>
        something went wrong, <br />
        go back or reload the page
      </p>
      <p>
        ...and <LinkWindow to={HUB_LINK}>let us know</LinkWindow> about this
      </p>

      <div className={styles.errorDetails}>
        <pre>{error.message}</pre>
        <pre className={styles.stack}>{error.stack}</pre>
      </div>

      <footer>
        {window.history.length > 0 && (
          <BtnGrd
            className={styles.btnGoBack}
            text="go back"
            onClick={() => {
              window.history.back();
              window.location.reload();
            }}
          />
        )}

        <BtnGrd text="reload page" onClick={() => window.location.reload()} />
      </footer>
    </div>
  );
}

export default ErrorScreen;

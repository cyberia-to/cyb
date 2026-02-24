import { AboutTeleport, BridgeAction, SendAction, SwapAction } from './components';
import styles from './TeleportMainScreen.module.scss';

function TeleportMainScreen() {
  return (
    <div className={styles.container}>
      <AboutTeleport />
      <div className={styles.cardContainer}>
        <SendAction />
        <BridgeAction />
        <SwapAction />
      </div>
    </div>
  );
}

export default TeleportMainScreen;

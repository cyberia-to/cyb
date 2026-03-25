import Logo from '../Logo/Logo';
import TeleportStat from '../TeleportStat/TeleportStat';
import TeleportText from '../TeleportText/TeleportText';
import styles from './AboutTeleport.module.scss';

function AboutTeleport() {
  return (
    <div className={styles.container}>
      <h1 className={styles.title}>teleport</h1>
      <Logo />
      <TeleportText />
      <TeleportStat />
    </div>
  );
}

export default AboutTeleport;

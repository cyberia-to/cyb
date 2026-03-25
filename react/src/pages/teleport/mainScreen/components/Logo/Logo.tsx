import styles from './Logo.module.scss';

function Logo() {
  return (
    <div className={styles.container}>
      <div className={styles.teleportLogo} />
    </div>
  );
}

export default Logo;

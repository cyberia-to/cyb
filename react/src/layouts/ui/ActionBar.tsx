import Commander from 'src/containers/application/Header/Commander/Commander';
import styles from './ActionBar.module.scss';

function ActionBarContainer({ children }) {
  return (
    <div className={styles.ActionBarContainer}>
      <div className={styles.ActionBarContainerContent}>{children}</div>
    </div>
  );
}

function ActionBar() {
  return (
    <ActionBarContainer>
      <Commander />
      <div id="portalActionBar" />
    </ActionBarContainer>
  );
}

export default ActionBar;

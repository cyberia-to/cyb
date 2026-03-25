import styles from './styles.scss';

const STEP_INIT = 0;

function MoonAnimation({ stepCurrent }: { stepCurrent?: number }) {
  return (
    <div
      className={styles.mask}
      style={{
        right: stepCurrent === STEP_INIT ? '50%' : '7%',
      }}
    >
      <div className={styles.mask__inner} />
    </div>
  );
}

export default MoonAnimation;

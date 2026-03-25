import { Display, DisplayTitle } from 'src/components';
import styles from './UseDesktopVersionBlock.module.scss';

function UseDesktopVersionBlock() {
  return (
    <div className={styles.wrapper}>
      <Display title={<DisplayTitle title="page not available" />} color="red">
        this page not available on mobile <br /> use desktop version for better experience
      </Display>
    </div>
  );
}

export default UseDesktopVersionBlock;

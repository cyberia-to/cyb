import { useAppSelector } from 'src/redux/hooks';
import styles from './Header.module.scss';
import HeaderSwapItem from './ui/HeaderSwapItem/HeaderSwapItem';
import HeaderItem from './ui/HeaderItem/HeaderItem';
import HeaderIbcItem from './ui/HeaderIbcItem/HeaderIbcItem';
import PackageSelected from './ui/PackageSelected/PackageSelected';

function Header() {
  return (
    <div className={styles.containerHeader}>
      <HeaderItem>
        <span>choose token to spend</span>
      </HeaderItem>
      <HeaderItem>
        <HeaderSwapItem />
      </HeaderItem>
      <HeaderItem>
        <HeaderIbcItem />
      </HeaderItem>
    </div>
  );
}

export default Header;

import cx from 'classnames';
import styles from './BurgerIcon.module.scss';

function BurgerIcon({ openMenu }: { openMenu: boolean }) {
  return (
    <div
      className={cx(styles.networkBtnImgIconMenu, {
        [styles.networkBtnImgIconMenuClose]: !openMenu,
      })}
    >
      <div />
      <div />
      <div />
    </div>
  );
}

export default BurgerIcon;

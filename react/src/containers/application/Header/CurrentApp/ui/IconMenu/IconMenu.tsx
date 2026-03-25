import cx from 'classnames';
import styles from './IconMenu.module.scss';

function IconMenu({ openMenu }: { openMenu: boolean }) {
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

export default IconMenu;

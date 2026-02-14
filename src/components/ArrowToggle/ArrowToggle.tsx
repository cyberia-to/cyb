import arrowImg from 'images/Line22.svg';
import cx from 'classnames';
import styles from './ArrowToggle.module.scss';

function ArrowToggle({ isOpen }: { isOpen: boolean }) {
  return (
    <img
      alt="img-ArrowToggle"
      src={arrowImg}
      className={cx(styles.btnOpenIconArrowImg, {
        [styles.btnOpenIconArrowImgOpen]: isOpen,
      })}
    />
  );
}

export default ArrowToggle;

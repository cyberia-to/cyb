import line22 from '../../../../../image/Line22.svg';
import styles from './styles.scss';

const cx = require('classnames');

// TODO: remove component
function BtnArrow({ disabled, onClick, open }) {
  return (
    <div
      // type="button"
      // disabled={disabled}
      // onClick={onClick}
      className={styles.btnContainerBtnArrow}
    >
      <img
        src={line22}
        alt="img"
        className={cx(styles.btnContainerBtnArrowImg, {
          [styles.btnContainerBtnArrowImgOpen]: open,
        })}
      />
    </div>
  );
}

export default BtnArrow;

import styles from './styleBtnPasport.scss';

const cx = require('classnames');

function GradientContainer({ styleContent, children }) {
  return (
    <div
      className={cx(styles.GradientContainer, {
        [styles.GradientContainerBlue]: styleContent === 'blue',
        [styles.GradientContainerDanger]: styleContent === 'red',
      })}
    >
      {children}
    </div>
  );
}

function BtnPasport({ children, typeBtn, ...props }) {
  return (
    <button
      className={cx(styles.BtnPasport, {
        [styles.BtnPasportBlue]: typeBtn === 'blue',
        [styles.BtnPasportRed]: typeBtn === 'red',
      })}
      type="button"
      {...props}
    >
      <GradientContainer styleContent={typeBtn}>{children}</GradientContainer>
    </button>
  );
}

export default BtnPasport;

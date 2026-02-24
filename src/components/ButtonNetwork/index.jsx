import styles from './styles.scss';

function ButtonNetwork({ onClick, disabled, network }) {
  return (
    <button type="button" disabled={disabled} onClick={onClick} className={styles.buttonNetwork}>
      {network}
    </button>
  );
}

// eslint-disable-next-line import/no-unused-modules
export default ButtonNetwork;

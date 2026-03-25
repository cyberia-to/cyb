import styles from './NewThreadBtn.module.scss';

function NewThreadBtn({ onClick }) {
  return (
    <button onClick={onClick} className={styles.btn} type="button">
      <div>+</div>
      <h3>new conversation</h3>
      <p>click here to start</p>
    </button>
  );
}

export default NewThreadBtn;

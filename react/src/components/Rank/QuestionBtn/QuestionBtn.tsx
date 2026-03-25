import styles from './QuestionBtn.module.scss';

// temp, need split to icon I think
function QuestionBtn({ ...props }) {
  return (
    <button type="button" className={styles.questionBtn} {...props}>
      ?
    </button>
  );
}

export default QuestionBtn;

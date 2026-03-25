import styles from './stylesCode.scss';

function CodeSnipet({ src }) {
  return (
    <pre className={styles.containerPre}>
      <code className={styles.containerPreCode}>{src}</code>
    </pre>
  );
}

export default CodeSnipet;

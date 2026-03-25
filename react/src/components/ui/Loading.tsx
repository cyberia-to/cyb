import styles from './Loading.module.scss';

function Loading() {
  return (
    <div className={styles.container}>
      <div className={styles['lds-grid']}>
        {Array.from(Array(9)).map((_, index) => (
          <div key={index} />
        ))}
      </div>
    </div>
  );
}

export default Loading;

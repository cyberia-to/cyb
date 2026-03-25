import { Dots } from './Dots';
import styles from './Loading.module.scss';

// temp
function Loader2({ text = 'loading' }: { text?: string }) {
  return (
    <div className={styles.container}>
      <p>{text}</p> <Dots />
    </div>
  );
}

export default Loader2;

import { ReactNode } from 'react';
import styles from './Row.module.scss';

function Row({ value, title }: { title: ReactNode; value: ReactNode }) {
  return (
    <div className={styles.container}>
      <span className={styles.key}>{title}:</span>
      <span className={styles.value}>{value}</span>
    </div>
  );
}

export function RowsContainer({ children }: { children: ReactNode }) {
  return <div className={styles.rowsContainer}>{children}</div>;
}

export default Row;

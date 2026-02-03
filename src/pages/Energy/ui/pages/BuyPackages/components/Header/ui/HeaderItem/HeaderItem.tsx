import { ReactNode } from 'react';
import styles from './HeaderItem.module.scss';

function HeaderItem({ children }: { children: ReactNode }) {
  return (
    <div className={styles.itemHeader}>
      <span className={styles.content}>{children}</span>
    </div>
  );
}

export default HeaderItem;

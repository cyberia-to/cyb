import styles from './AccountInput.module.scss';

export default function AccountInputListContainer({ children }: { children: React.ReactNode }) {
  return <div className={styles.containerList}>{children}</div>;
}

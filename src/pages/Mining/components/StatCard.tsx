import styles from '../Mining.module.scss';

type Props = {
  label: string;
  value: string | number;
  suffix?: string;
  children?: React.ReactNode;
};

function StatCard({ label, value, suffix, children }: Props) {
  return (
    <div className={styles.statCard}>
      <span className={styles.statCardLabel}>{label}</span>
      <span className={styles.statCardValue}>
        {value}
        {suffix && <span className={styles.statCardSuffix}> {suffix}</span>}
      </span>
      {children}
    </div>
  );
}

export default StatCard;

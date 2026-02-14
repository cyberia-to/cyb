import styles from '../Mining.module.scss';

type Props = {
  label: string;
  value: string | number;
  suffix?: string;
};

function StatCard({ label, value, suffix }: Props) {
  return (
    <div className={styles.statCard}>
      <span className={styles.statCardLabel}>{label}</span>
      <span className={styles.statCardValue}>
        {value}
        {suffix && <span className={styles.statCardSuffix}> {suffix}</span>}
      </span>
    </div>
  );
}

export default StatCard;

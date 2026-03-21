import styles from '../Mining.module.scss';

type Props = {
  value: number;
  onChange: (n: number) => void;
  max: number;
  total: number;
  disabled?: boolean;
};

function ThreadSelector({ value, onChange, max, total, disabled }: Props) {
  return (
    <div className={styles.threadSelector}>
      <span className={styles.threadLabel}>
        {value}/{total} cores
      </span>
      <input
        type="range"
        min={1}
        max={max}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        disabled={disabled}
        className={styles.threadRange}
      />
    </div>
  );
}

export default ThreadSelector;

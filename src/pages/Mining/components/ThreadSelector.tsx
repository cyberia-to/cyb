import styles from '../Mining.module.scss';

type Props = {
  value: number;
  onChange: (n: number) => void;
  max: number;
  disabled?: boolean;
};

function ThreadSelector({ value, onChange, max, disabled }: Props) {
  return (
    <div className={styles.threadSelector}>
      <span className={styles.threadLabel}>
        {value} / {max} cores
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

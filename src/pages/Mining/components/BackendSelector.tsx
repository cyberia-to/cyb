import styles from '../Mining.module.scss';

type Props = {
  value: string;
  onChange: (backend: string) => void;
  availableBackends: string[];
  activeBackend?: string;
  disabled?: boolean;
};

const BACKEND_LABELS: Record<string, string> = {
  cpu: 'CPU',
  metal: 'Metal GPU',
  cuda: 'CUDA GPU',
  wgpu: 'WGPU GPU',
};

function BackendSelector({
  value,
  onChange,
  availableBackends,
  activeBackend,
  disabled,
}: Props) {
  return (
    <div className={styles.threadSelector}>
      <span className={styles.threadLabel}>
        Backend
        {activeBackend && value === 'auto' ? ` (${activeBackend})` : ''}
      </span>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        className={styles.backendSelect}
      >
        {availableBackends.map((b) => (
          <option key={b} value={b}>
            {BACKEND_LABELS[b] || b}
          </option>
        ))}
      </select>
    </div>
  );
}

export default BackendSelector;

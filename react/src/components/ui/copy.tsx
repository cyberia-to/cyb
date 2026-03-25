import styles from './Copy.module.scss';

// eslint-disable-next-line import/prefer-default-export
export function Copy({ text, ...props }) {
  return (
    <button
      className={styles.copyBtn}
      type="button"
      aria-label="Save"
      {...props}
      onClick={() => {
        navigator.clipboard.writeText(text);
        // Auto-clear clipboard after 30 seconds to prevent leaking addresses
        setTimeout(() => {
          navigator.clipboard.writeText('').catch(() => {});
        }, 30000);
      }}
    />
  );
}

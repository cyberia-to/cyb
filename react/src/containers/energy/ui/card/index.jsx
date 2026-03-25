import styles from './styles.scss';

function Card({ title, value, stylesContainer, active, ...props }) {
  return (
    <div
      style={{ ...stylesContainer }}
      className={`${styles.containerCard} ${active ? styles.containerCardActive : ''}`}
      {...props}
    >
      <span className={styles.cardTitle}>{value}</span>
      <span className={styles.cardValue}>{title}</span>
    </div>
  );
}

export default Card;

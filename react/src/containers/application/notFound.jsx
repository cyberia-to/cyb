import styles from './notFound.module.scss';

function NotFound({ text }) {
  return <div className={styles.container}>{text || 'not found'}</div>;
}

export default NotFound;

import styles from './steles.scss';

function Img({ content }) {
  return <img className={styles.img} alt="img" src={content} />;
}

export default Img;

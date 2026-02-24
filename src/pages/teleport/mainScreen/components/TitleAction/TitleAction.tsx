import { Link } from 'react-router-dom';
import DisplayTitle from 'src/components/containerGradient/DisplayTitle/DisplayTitle';
import styles from './TitleAction.module.scss';

function TitleAction({ title, subTitle, to }: { title: string; subTitle: string; to: string }) {
  return (
    <DisplayTitle
      inDisplay
      title={
        <Link to={to} className={styles.container}>
          <h3 className={styles.title}>{title}</h3>
          <span className={styles.subTitle}>{subTitle}</span>
        </Link>
      }
    />
  );
}

export default TitleAction;

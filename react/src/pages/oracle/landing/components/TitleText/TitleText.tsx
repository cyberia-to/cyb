import { Link } from 'react-router-dom';
import { routes } from 'src/routes';
import styles from './TitleText.module.scss';

type Props = {
  title: string;
  text: string | JSX.Element;
};

function TitleText({ title, text }: Props) {
  return (
    <div className={styles.wrapper}>
      <Link to={routes.search.getLink(title)}>
        <h6>{title}</h6>
      </Link>
      <p>{text}</p>
    </div>
  );
}

export default TitleText;

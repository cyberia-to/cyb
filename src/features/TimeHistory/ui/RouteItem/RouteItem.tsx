import DEFAULT_IMG from 'images/large-orange-circle.png';
import { Link } from 'react-router-dom';
import getMenuItems from 'src/utils/appsMenu/appsMenu';
import findApp from 'src/utils/findApp';
import { RouteItemT } from '../type';
import styles from './RouteItem.module.scss';

function RouteItem({ value }: { value: RouteItemT }) {
  const { url } = value;
  const app = findApp(getMenuItems(), url);

  return (
    <Link to={url} className={styles.wrapper}>
      <span>{app[0]?.name || url}</span>
      <img src={app[0]?.icon || DEFAULT_IMG} alt={url} />
    </Link>
  );
}

export default RouteItem;

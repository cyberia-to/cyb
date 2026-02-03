import { plans } from 'src/pages/Energy/types/type';
import { useAppSelector } from 'src/redux/hooks';
import cx from 'classnames';
import styles from './PackageSelected.module.scss';

function PackageSelected() {
  const { selectPlan } = useAppSelector((state) => state.energy);

  const findPlan =
    selectPlan && plans.find((item) => item.price === selectPlan.keyPackage);

  if (!findPlan) {
    return <span>package</span>;
  }

  return (
    <span className={styles.wrapper}>
      package
      <img
        className={styles.packageImg}
        src={findPlan.icon}
        alt="findPlanImg"
      />
      <span className={cx(styles.packageName, styles[findPlan.color])}>
        {findPlan.name}
      </span>
    </span>
  );
}

export default PackageSelected;

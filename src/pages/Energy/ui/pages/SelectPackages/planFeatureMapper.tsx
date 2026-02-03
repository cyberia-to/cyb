import styles from './Buy.module.scss';
import checkmark from './images/checkmark.svg';
import IconsNumber from 'src/components/IconsNumber/IconsNumber';
import { plans, features } from '../../../types/type';
import portal from 'images/space-pussy.svg';
import oracle from 'src/image/new_icons/oracle.svg';
import docs from 'src/image/new_icons/docs.svg';
import sphere from 'src/image/new_icons/sphere.svg';
import hfr from 'src/image/new_icons/hfr.svg';
import teleport from 'src/image/new_icons/teleport.svg';
import robot from 'src/image/new_icons/robot.svg';

export const renderFeatureContent = (feature: (typeof features)[0]) => {
  if (feature.label === '3 free to use aips') {
    return (
      <div className={styles.icons}>
        <img src={portal} alt="portal" width="20" height="20" />
        <img src={oracle} alt="oracle" width="20" height="20" />
        <img src={docs} alt="docs" width="20" height="20" />
      </div>
    );
  }
  if (feature.label === 'all powered aips') {
    return (
      <div className={styles.icons}>
        <img src={sphere} alt="sphere" width="20" height="20" />
        <img src={hfr} alt="hfr" width="20" height="20" />
        <img src={teleport} alt="teleport" width="20" height="20" />
        <img src={robot} alt="robot" width="20" height="20" />
      </div>
    );
  }
  return null;
};

export const renderPlanFeature = (
  plan: (typeof plans)[0],
  featureIndex: number
) => {
  const featureRenderers = {
    default: () => (
      <div
        className={
          plan.features[featureIndex] ? styles.checkmark : styles.excluded
        }
      >
        {plan.features[featureIndex] ? (
          <img src={checkmark} alt="checkmark" className={styles.checkmark} />
        ) : (
          '❌'
        )}
      </div>
    ),
    2: () =>
      plan.uploads > 0 ? (
        <div className={styles.uploads}>~ {plan.uploads}</div>
      ) : (
        <div className={styles.excluded}>❌</div>
      ),
    3: () => (
      <div className={styles.symbols}>
        <span className={styles.symbolsText}>
          {'> '}
          <span className={styles.symbolsDigit}>{plan.symbols}</span>
          {' symbols'}
        </span>
      </div>
    ),
    4: () =>
      plan.fuel > 0 ? (
        <div className={styles.fuel}>
          <IconsNumber value={plan.fuel} type="hydrogen" isVertical={false} />
        </div>
      ) : (
        <div className={styles.excluded}>❌</div>
      ),
    5: () =>
      plan.energy > 0 ? (
        <div className={styles.energy}>
          <span className={styles.energyIcon}>
            <IconsNumber value={plan.energy} type="energy" isVertical={false} />
          </span>
        </div>
      ) : (
        <div className={styles.excluded}>❌</div>
      ),
    6: () =>
      plan.influence > 0 ? (
        <div className={styles.influence}>
          {plan.influence} <span className={styles.gray}>‰</span>
        </div>
      ) : (
        <div className={styles.excluded}>❌</div>
      ),
  };

  const renderer =
    featureRenderers[featureIndex as keyof typeof featureRenderers] ||
    featureRenderers.default;
  return renderer();
};

import styles from './Buy.module.scss';
import { features, plans } from '../../../types/type';
import AdviserHoverWrapper from 'src/features/adviser/AdviserHoverWrapper/AdviserHoverWrapper';
import { Display } from 'src/components';
import voltImg from 'images/lightning2.png';
import amperImg from 'images/light.png';
import hydrogen from 'images/hydrogen.svg';
import { useMemo } from 'react';
import { renderPlanFeature } from './planFeatureMapper';

interface PlanDisplayProps {
  plan: (typeof plans)[0];
  index: number;
  isSelected: boolean;
}

function PlanDisplay({ plan, index, isSelected }: PlanDisplayProps) {
  const adviserContent = useMemo(() => {
    return (
      <div className={styles.adviserContent}>
        you'll get <span className={styles['blue-lights']}>{plan.volts}</span>
        <img src={voltImg} alt="lightning bolt" width="15" height="15" /> will
        tokens, <span className={styles['blue-lights']}>{plan.ampers}</span>
        <img src={amperImg} alt="light bulb" width="15" height="15" /> attention
        tokens with total{' '}
        <span className={styles['blue-lights']}>{plan.energy} </span> kW of
        linking power,{' '}
        <span className={styles['blue-lights']}>{plan.fuel}</span>
        <img src={hydrogen} alt="hydrogen" width="15" height="15" /> hydrogen to
        play with
      </div>
    );
  }, [plan]);

  return (
    <div
      className={`${styles.displayWrapper} ${isSelected ? styles.active : ''} ${
        styles[plan.color]
      }`}
      style={{ left: `${-2 * (index + 1)}px` }}
    >
      <Display
        color={plan.color}
        title={
          <div className={styles.planTitleWrapper}>
            <img
              src={plan.icon}
              alt={`${plan.name} icon`}
              className={styles.planIcon}
            />
            <span
              className={`${styles.planName} ${
                plan.name === 'ghost' ? styles.gray : styles[plan.color]
              }`}
            >
              {plan.name}
            </span>
          </div>
        }
        noPadding
      >
        <AdviserHoverWrapper adviserContent={adviserContent}>
          <div className={styles.planContent}>
            {features.map((_, featureIndex) => (
              <div key={featureIndex} className={styles.row}>
                {renderPlanFeature(plan, featureIndex)}
              </div>
            ))}
            <div className={styles.row}>
              <div
                className={`${styles.price} ${
                  plan.price === 'free' ? styles.gray : styles[plan.color]
                }`}
              >
                {plan.price}
                {plan.price === 'free' ? '' : '$'}
              </div>
            </div>
          </div>
        </AdviserHoverWrapper>
      </Display>
    </div>
  );
}

export default PlanDisplay;

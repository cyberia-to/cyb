import { useCallback, useMemo, useState } from 'react';
import { ActionBar, Button } from 'src/components';
import { useAppDispatch } from 'src/redux/hooks';
import { setSelectPlan } from 'src/pages/Energy/redux/energy.redux';
import { features, plans } from 'src/pages/Energy/types/type';
import { useEnergy } from 'src/pages/Energy/context/Energy.context';
import PlanDisplay from './PlanDisplay';
import styles from './Buy.module.scss';
import mapPlan from './utils';
import { renderFeatureContent } from './planFeatureMapper';
import { log } from 'tone/build/esm/core/util/Debug';

function SelectPackages() {
  const { energyPackageSwapRoutes } = useEnergy();
  const [selectedPlan, setSelectedPlan] = useState<string | null>(null);
  const dispatch = useAppDispatch();

  const onClickSelectPlan = useCallback(() => {
    console.log('selectedPlan', selectedPlan);
    console.log('energyPackageSwapRoutes', energyPackageSwapRoutes);
    if (!energyPackageSwapRoutes) {
      return;
    }

    const selectedPlanFind = energyPackageSwapRoutes.find(
      (item) => item.keyPackage === selectedPlan
    );

    if (!selectedPlanFind) {
      return;
    }

    dispatch(
      setSelectPlan({
        keyPackage: selectedPlanFind.keyPackage,
        tokenIn: selectedPlanFind.tokenIn,
      })
    );
  }, [dispatch, selectedPlan, energyPackageSwapRoutes]);

  const renderPlans = useMemo(() => {
    return plans.map((plan, index) => {
      if (energyPackageSwapRoutes) {
        const findItem = energyPackageSwapRoutes.find(
          (item) => item.keyPackage === plan.price
        );

        const { fuel, energy, uploads, symbols, ampers, volts } =
          mapPlan(findItem);

        console.log('Plan details:', {
          fuel: fuel.toNumber(),
          energy: energy.toNumber(),
          uploads: uploads.toNumber(),
          symbols,
          ampers: ampers.toNumber(),
          volts: volts.toNumber(),
        });
        plan.fuel = fuel.toNumber();
        console.debug('plan.fuel', plan.fuel);
        plan.energy = energy.toNumber();
        plan.uploads = uploads.toNumber();
        plan.symbols = symbols;
        plan.ampers = ampers.toNumber();
        plan.volts = volts.toNumber();
      }
      return (
        <button
          type="button"
          key={plan.name}
          onClick={() => setSelectedPlan(plan.price)}
          className={styles.planWrapper}
        >
          <PlanDisplay
            plan={plan}
            index={index}
            isSelected={selectedPlan === plan.price}
          />
        </button>
      );
    });
  }, [energyPackageSwapRoutes, selectedPlan]);

  return (
    <>
      <div className={styles.buyContainer}>
        <div className={styles.labelsColumn}>
          <div className={styles.planContent}>
            {features.map((feature, index) => (
              <div key={index} className={styles.row}>
                <div className={styles.labelGroup}>
                  <div className={styles.label}>{feature.label}</div>
                  {feature.subLabel && (
                    <div className={styles.subLabel}>{feature.subLabel}</div>
                  )}
                  {renderFeatureContent(feature)}
                </div>
              </div>
            ))}
          </div>
        </div>
        {renderPlans}
      </div>

      {selectedPlan && (
        <ActionBar>
          <Button
            onClick={() => onClickSelectPlan()}
            className={styles.actionBarBtn}
            disabled={selectedPlan === 'free'}
          >
            Buy {plans.find((item) => item.price === selectedPlan)?.name} Plan
          </Button>
        </ActionBar>
      )}
    </>
  );
}

export default SelectPackages;

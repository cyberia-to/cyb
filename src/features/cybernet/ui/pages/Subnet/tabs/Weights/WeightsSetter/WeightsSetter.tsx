import { useEffect, useState } from 'react';
import { ActionBar, InputNumber } from 'src/components';
import { sessionStorageKeys } from 'src/constants/sessionStorageKeys';
import { useSubnet } from '../../../subnet.context';
import styles from './WeightsSetter.module.scss';

const DEFAULT_WEIGHT = 5;

const sessionStorageKey = sessionStorageKeys.subnetWeights;

type Props = {
  callback: () => void;
};

function getSSData() {
  const data = sessionStorage.getItem(sessionStorageKey);

  return data ? JSON.parse(data) : null;
}

function WeightsSetter({ callback, weights: w }: Props) {
  const { subnetQuery, neuronsQuery } = useSubnet();

  const w2 = w?.reduce((acc, [uid, value], _i) => {
    acc[uid] = value;
    return acc;
  }, {});

  const { max_weights_limit: maxWeightsLimit, subnetwork_n: length, netuid } = subnetQuery.data!;

  const neurons = neuronsQuery.data || [];

  const _ssData = getSSData()?.[netuid];

  const [weights, setWeights] = useState(
    // ssData ||
    new Array(length)
      .fill(DEFAULT_WEIGHT)
      .map((_, i) => (w2?.[i] ? (w2[i] / maxWeightsLimit) * 10 : 0).toFixed())
  );

  useEffect(() => {
    return () => {
      sessionStorage.setItem(
        sessionStorageKey,
        JSON.stringify({
          [netuid]: weights,
        })
      );
    };
  }, [netuid, weights]);

  return (
    <div className={styles.wrapper}>
      <div className={styles.group} ref={ref}>
        <span>change grades</span>
        {new Array(length).fill(null).map((_, i) => {
          const { hotkey } = neurons[i];

          const value = weights[i];
          return (
            <div key={i}>
              <InputNumber
                data-address={hotkey}
                maxValue={10}
                value={value || (value === null ? 0 : value)}
                onChange={(e) => {
                  const newWeights = [...weights];
                  newWeights[i] = +e;
                  setWeights(newWeights);
                }}
              />
            </div>
          );
        })}
      </div>

      <ActionBar
        button={{
          text: 'Submit grades',
          onClick: submit,
          disabled: isLoading,
        }}
      />
    </div>
  );
}

export default WeightsSetter;

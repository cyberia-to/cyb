import { useEffect, useRef } from 'react';
import { InputNumber } from 'src/components';
import { usePreviousPage } from 'src/contexts/previousPage';
import { useSubnet } from '../subnet.context';

type Props = {
  uid: number;
};

function GradeSetterInput({ uid }: Props) {
  const {
    subnetQuery,
    neuronsQuery: { data: neurons },
    grades: { newGrades },
  } = useSubnet();

  const rootSubnet = subnetQuery.data?.netuid === 0;

  const { previousPathname } = usePreviousPage();

  const ref = useRef<HTMLDivElement>(null);

  const handled = useRef(false);

  // need this because autoFocus not updateable
  // bullshit, need refactor
  useEffect(() => {
    if (handled.current === true) {
      return;
    }

    if (rootSubnet) {
      return;
    }

    const search = new URLSearchParams(previousPathname?.split('?')[1]);
    const neuron = search.get('neuron');

    const hothey = neurons?.find((n) => n.uid === uid)?.hotkey;

    if (ref.current && neuron === hothey) {
      ref.current.querySelector('input')?.focus();
      handled.current = true;
    }
  }, [previousPathname, neurons, rootSubnet, uid]);

  return (
    <div ref={ref}>
      <InputNumber
        // disabled={!rootSubnet && !neurons?.[uid]?.validator_permit}
        key={uid}
        maxValue={10}
        value={newGrades?.data?.[uid] || 0}
        onChange={(e) => {
          newGrades?.setGrade(uid.toString(), +e);
        }}
      />

      {/* <Slider
        min={0}
        max={10}
        onChange={(amount: number) => {
          newGrades?.setGrade(uid.toString(), amount);
        }}
        value={newGrades?.data?.[uid] || 0}
        railStyle={{
          backgroundColor: 'red',
        }}
        marks={{
          0: '0',
          5: '5',
          10: '10',
        }}
      /> */}
    </div>
  );
}

export default GradeSetterInput;

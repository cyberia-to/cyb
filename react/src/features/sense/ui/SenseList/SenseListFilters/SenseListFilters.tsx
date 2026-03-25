import ButtonsGroup from 'src/components/buttons/ButtonsGroup/ButtonsGroup';
import { Filters } from '../../types';

type Props = {
  selected: Filters;
  onChangeFilter: (filter: Filters) => void;
};

const config = {
  [Filters.All]: {
    label: 'All',
    tooltip: 'All',
  },
  [Filters.Particle]: {
    label: '#',
    tooltip: 'Particle',
  },
  [Filters.Neuron]: {
    label: '@',
    tooltip: 'Neuron',
  },
  [Filters.LLM]: {
    label: 'llm',
    tooltip: 'LLM',
  },
};

function SenseListFilters({ selected, onChangeFilter }: Props) {
  return (
    <ButtonsGroup
      type="radio"
      items={Object.keys(config).map((key) => {
        const { label, tooltip } = config[key as Filters];
        return {
          label,
          name: key,
          checked: selected === +key,
          tooltip,
        };
      })}
      onChange={(value) => onChangeFilter(Number(value) as Filters)}
    />
  );
}

export default SenseListFilters;

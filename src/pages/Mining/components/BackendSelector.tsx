import Select, { SelectOption } from 'src/components/Select';
import { Color } from 'src/components/LinearGradientContainer/LinearGradientContainer';
import { useMemo } from 'react';

type Props = {
  value: string;
  onChange: (backend: string) => void;
  availableBackends: string[];
  activeBackend?: string;
  disabled?: boolean;
};

const BACKEND_LABELS: Record<string, string> = {
  cpu: 'CPU',
  metal: 'Metal GPU',
  cuda: 'CUDA GPU',
  wgpu: 'WGPU GPU',
};

function BackendSelector({
  value,
  onChange,
  availableBackends,
  activeBackend,
  disabled,
}: Props) {
  const options: SelectOption[] = useMemo(
    () =>
      availableBackends.map((b) => ({
        value: b,
        text: BACKEND_LABELS[b] || b,
      })),
    [availableBackends]
  );

  const currentLabel = BACKEND_LABELS[value] || value;
  const suffix =
    activeBackend && value === 'auto' ? ` (${activeBackend})` : '';

  return (
    <Select
      valueSelect={value}
      onChangeSelect={onChange}
      options={options}
      currentValue={`${currentLabel}${suffix}`}
      color={Color.Green}
      disabled={disabled}
      width="140px"
      small
    />
  );
}

export default BackendSelector;

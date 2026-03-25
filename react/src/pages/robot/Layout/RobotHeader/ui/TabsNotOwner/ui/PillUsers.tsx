import IconsNumber from 'src/components/IconsNumber/IconsNumber';
import Pill from 'src/components/Pill/Pill';
import { formatNumber } from 'src/utils/utils';

type Props = {
  name: string;
  value: number | string;
};

function PillUsers({ name, value }: Props) {
  if (name === 'sigma') {
    return <IconsNumber value={value} type="hydrogen" />;
  }

  return <Pill text={formatNumber(value)} />;
}

export default PillUsers;

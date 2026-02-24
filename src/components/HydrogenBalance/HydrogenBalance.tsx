import { useGetBalanceBostrom } from 'src/containers/sigma/hooks';
import IconsNumber from '../IconsNumber/IconsNumber';

type Props = {
  address?: string;
  isVertical?: boolean;
};

function HydrogenBalance({ address, isVertical }: Props) {
  const { totalAmountInLiquid } = useGetBalanceBostrom(address);

  return (
    <span>
      <IconsNumber value={totalAmountInLiquid.currentCap} type="hydrogen" isVertical={isVertical} />
    </span>
  );
}

export default HydrogenBalance;

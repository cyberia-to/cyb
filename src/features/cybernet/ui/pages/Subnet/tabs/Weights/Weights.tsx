import Display from 'src/components/containerGradient/Display/Display';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import WeightsTable from './WeightsTable/WeightsTable';

type Props = {};

function Weights({}: Props) {
  useAdviserTexts({
    defaultText: 'Subnet weights',
  });

  return (
    <div>
      <Display noPadding>
        <WeightsTable />
      </Display>

      <br />
    </div>
  );
}

export default Weights;

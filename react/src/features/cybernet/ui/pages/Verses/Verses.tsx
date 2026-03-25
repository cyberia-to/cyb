import Display from 'src/components/containerGradient/Display/Display';
import DisplayTitle from 'src/components/containerGradient/DisplayTitle/DisplayTitle';
import ContractsTable from '../Main/ContractsTable/ContractsTable';

function Verses() {
  return (
    <Display title={<DisplayTitle title="Verses" />} noPaddingX>
      <ContractsTable />
    </Display>
  );
}

export default Verses;

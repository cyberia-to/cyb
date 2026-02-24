import { Row, RowsContainer } from 'src/components';
import Display from 'src/components/containerGradient/Display/Display';
import DisplayTitle from 'src/components/containerGradient/DisplayTitle/DisplayTitle';
import { formatNumber, trimString } from '../../utils/utils';

const dateFormat = require('dateformat');

function InformationBlock({ data, numbTx }) {
  return (
    <Display color="blue" title={<DisplayTitle title="Information" />}>
      <RowsContainer>
        <Row value={data.height ? formatNumber(data.height) : ''} title="Height" />
        <Row
          value={data.timestamp ? dateFormat(data.timestamp, 'dd/mm/yyyy, HH:MM:ss') : ''}
          title="Block Time"
        />
        <Row value={data.hash ? trimString(data.hash, 6, 6) : ''} title="Block Hash" />
        <Row value={Object.keys(numbTx).length} title="Number of Transactions" />
      </RowsContainer>
    </Display>
  );
}

export default InformationBlock;

import { useMemo } from 'react';
import Table from 'src/components/Table/Table';
import { useHub } from 'src/contexts/hub';
import { entityToDto } from 'src/utils/dto';
import DisplayHub from '../../components/DisplayHub/DisplayHub';
import renderColumns from './map';

function Tokens() {
  const { tokens } = useHub();

  const dataRow = tokens ? Object.keys(tokens).map((key) => entityToDto(tokens[key])) : [];

  const columnsData = useMemo(() => renderColumns(), []);

  return (
    <DisplayHub title="tokens" type="HUB_TOKENS">
      <Table data={dataRow} columns={columnsData} />
    </DisplayHub>
  );
}

export default Tokens;

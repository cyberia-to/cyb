import { useMemo } from 'react';
import Table from 'src/components/Table/Table';
import { useHub } from 'src/contexts/hub';
import { entityToDto } from 'src/utils/dto';
import DisplayHub from '../../components/DisplayHub/DisplayHub';
import renderColumnsData from './map';

function Channels() {
  const { channels } = useHub();

  const dataRow = channels ? Object.keys(channels).map((key) => entityToDto(channels[key])) : [];

  const columnsData = useMemo(() => {
    return renderColumnsData();
  }, []);

  return (
    <DisplayHub title="channels" type="HUB_CHANNELS">
      <Table columns={columnsData} data={dataRow} />
    </DisplayHub>
  );
}

export default Channels;

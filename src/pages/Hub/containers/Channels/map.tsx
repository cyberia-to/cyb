import { createColumnHelper } from '@tanstack/react-table';
import { Dot } from 'src/components';
import { EntityToDto } from 'src/types/dto';
import { Channel } from 'src/types/hub';

const columnHelper = createColumnHelper<EntityToDto<Channel>>();

const renderColumnsData = () => {
  return [
    columnHelper.accessor('id', {
      header: 'id',
    }),
    columnHelper.accessor('active', {
      header: 'active',
      cell: (info) => <Dot color={info.getValue() ? 'green' : 'red'} animation />,
    }),
    columnHelper.accessor('destinationChainId', {
      header: 'destination chain id',
    }),
    columnHelper.accessor('destinationChannelId', {
      header: 'destination channel id',
    }),
    columnHelper.accessor('sourceChainId', {
      header: 'source chain id',
    }),
    columnHelper.accessor('sourceChannelId', {
      header: 'source channel id',
    }),
  ];
};

export default renderColumnsData;

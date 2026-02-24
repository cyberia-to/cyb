import { createColumnHelper } from '@tanstack/react-table';
import { Cid } from 'src/components';
import ImgDenom from 'src/components/valueImg/imgDenom';
import { EntityToDto } from 'src/types/dto';
import { Network } from 'src/types/hub';
import { trimString } from 'src/utils/utils';

const columnHelper = createColumnHelper<EntityToDto<Network>>();

const renderColumnsData = () => [
  columnHelper.accessor('id', {
    header: 'id',
  }),
  columnHelper.accessor('chainId', {
    header: 'chainId',
  }),
  columnHelper.accessor('name', {
    header: 'name',
  }),
  columnHelper.accessor('logo', {
    header: 'logo',
    cell: (logo) => (
      <ImgDenom coinDenom="" tooltipStatus={false} infoDenom={{ coinImageCid: logo.getValue() }} />
    ),
  }),
  columnHelper.accessor('genesisHash', {
    header: 'genesis hash',
    cell: (value) => {
      const cid = value.getValue();
      return <Cid cid={cid}>{trimString(cid, 6, 6)}</Cid>;
    },
  }),
  columnHelper.accessor('prefix', {
    header: 'prefix',
  }),
];

export default renderColumnsData;

import { createColumnHelper } from '@tanstack/react-table';
import ImgDenom from 'src/components/valueImg/imgDenom';
import { EntityToDto } from 'src/types/dto';
import { Token } from 'src/types/hub';
import { trimString } from 'src/utils/utils';

const columnHelper = createColumnHelper<EntityToDto<Token>>();

const renderColumns = () => [
  columnHelper.accessor('id', {
    header: 'id',
  }),
  columnHelper.accessor('contract', {
    header: 'contract',
    cell: (info) => {
      const denom = info.getValue();
      return denom.includes('ibc') ? trimString(denom, 9, 6) : denom;
    },
  }),
  columnHelper.accessor('channelId', {
    header: 'channel id',
  }),
  columnHelper.accessor('ticker', {
    header: 'ticker',
  }),
  columnHelper.accessor('logo', {
    header: 'logo',
    cell: (logo) => (
      <ImgDenom coinDenom="" tooltipStatus={false} infoDenom={{ coinImageCid: logo.getValue() }} />
    ),
  }),
  columnHelper.accessor('decimals', {
    header: 'decimals',
  }),
  columnHelper.accessor('chainId', {
    header: 'chain id',
  }),
];

export default renderColumns;

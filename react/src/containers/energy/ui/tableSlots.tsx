import { createColumnHelper } from '@tanstack/react-table';
import { useMemo } from 'react';
import Table from 'src/components/Table/Table';
import { DENOM_LIQUID } from 'src/constants/config';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { CssVariables } from 'src/style/variables';
import { DenomArr, TextTable } from '../../../components';
import { formatNumber, getDisplayAmount } from '../../../utils/utils';

const columnHelper = createColumnHelper<any>();

type Props = {
  data: any;
};

function TableSlots({ data }: Props) {
  const { tracesDenom } = useIbcDenom();
  const [{ coinDecimals: coinDecimalsA }] = tracesDenom('milliampere');
  const [{ coinDecimals: coinDecimalsV }] = tracesDenom('millivolt');

  return (
    <Table
      columns={useMemo(
        () => [
          columnHelper.accessor('state', {
            header: 'State',
            cell: (info) => info.getValue(),
          }),
          columnHelper.accessor('unfreezing', {
            header: 'Unfreezing',
            cell: (info) => info.getValue(),
          }),
          columnHelper.accessor('supplied', {
            header: 'Supplied',
            cell: (info) => info.getValue(),
          }),
          columnHelper.accessor('received', {
            header: 'Received',
            cell: (info) => info.getValue(),
          }),
        ],
        []
      )}
      data={data?.map((item) => {
        return {
          state:
            item.status === 'Unfreezing' ? (
              <TextTable color={CssVariables.PRIMARY_COLOR}>Unfreezing</TextTable>
            ) : (
              <TextTable marginRight={5} color="#ff9100">
                Liquid
              </TextTable>
            ),
          unfreezing: item.time ? <TextTable>{item.time}</TextTable> : '',
          supplied: item.amount[DENOM_LIQUID] ? (
            <TextTable>
              {formatNumber(item.amount[DENOM_LIQUID])}&nbsp;
              <DenomArr denomValue={DENOM_LIQUID} onlyImg />
            </TextTable>
          ) : (
            ''
          ),
          received: (
            <>
              {item.amount.millivolt && (
                <TextTable>
                  {getDisplayAmount(item.amount.millivolt, coinDecimalsV)}&nbsp;
                  <DenomArr denomValue="millivolt" onlyImg />
                </TextTable>
              )}
              {item.amount.milliampere && (
                <TextTable>
                  {getDisplayAmount(item.amount.milliampere, coinDecimalsA)}
                  &nbsp;
                  <DenomArr denomValue="milliampere" onlyImg />
                </TextTable>
              )}
            </>
          ),
        };
      })}
    />
  );
}

export default TableSlots;

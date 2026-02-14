import BigNumber from 'bignumber.js';
import { ValidatorTableData } from 'src/pages/Sphere/types/tableData';
import IconsNumber from 'src/components/IconsNumber/IconsNumber';
import Table from 'src/components/Table/Table';
import TransitionContainer from '../TransitionContainer/TransitionContainer';

function ValidatorTableByGroup({
  data,
  columns,
  onSelect,
}: {
  data: { [key: string]: ValidatorTableData[] };
  columns: [];
  onSelect: (row?: ValidatorTableData) => void;
}) {
  return (
    <>
      {Object.keys(data).map((key) => {
        const itemData = data[key];
        const bondAmount = itemData.reduce(
          (acc, item) => {
            const amount = new BigNumber(acc.amount)
              .plus(item.delegation?.amount || 0)
              .toNumber();

            acc.amount = amount;
            if (item.delegation) {
              acc.denom = item.delegation.denom;
            }
            return acc;
          },
          { amount: 0, denom: '' }
        );

        return (
          <TransitionContainer
            key={key}
            title={key}
            isOpenState={!(key === 'inactive' || key === 'relax')}
            titleOptions={
              bondAmount.denom.length ? (
                <IconsNumber
                  value={bondAmount.amount}
                  type={bondAmount.denom}
                />
              ) : undefined
            }
          >
            <Table
              data={itemData}
              columns={columns}
              hideHeader
              onSelect={(row) => {
                onSelect(row ? itemData[Number(row)] : undefined);
              }}
            />
          </TransitionContainer>
        );
      })}
    </>
  );
}

export default ValidatorTableByGroup;

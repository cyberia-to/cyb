import Table from 'src/components/Table/Table';
import { ValidatorTableData } from 'src/pages/Sphere/types/tableData';
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

        return (
          <TransitionContainer
            key={key}
            title={key}
            isOpenState={!(key === 'inactive' || key === 'relax')}
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

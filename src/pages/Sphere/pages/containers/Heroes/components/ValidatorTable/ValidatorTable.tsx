import { useCallback, useMemo, useState } from 'react';
import _ from 'lodash';
import { Display, Tabs } from 'src/components';
import Table from 'src/components/Table/Table';
import BigNumber from 'bignumber.js';
import { ValidatorTableData } from '../../../../../types/tableData';
import renderColumnsData from './utils/mapValidatorTable';
import ValidatorTableByGroup from './ui/ValidatorTableByGroup/ValidatorTableByGroup';

enum TabsKey {
  power = 'power',
  bond = 'bond',
  apr = 'apr',
}

const TabsKeyConfig = {
  [TabsKey.power]: {
    title: 'power',
  },
  [TabsKey.apr]: {
    title: 'apr',
  },
  [TabsKey.bond]: {
    title: 'my heroes',
  },
};

function ValidatorTable({
  data,
  onSelectRow,
}: {
  data: ValidatorTableData[];
  onSelectRow: (row?: ValidatorTableData) => void;
}) {
  const [selected, setSelected] = useState<TabsKey>(TabsKey.power);
  const columns = useMemo(() => renderColumnsData(), []);

  const dataGroup = _.groupBy(data, (item) => item.rank);

  const dataBySelected = useMemo(() => {
    if (selected === TabsKey.bond) {
      return data.filter(
        (item) =>
          item.delegation && new BigNumber(item.delegation.amount).comparedTo(0)
      );
    }

    if (selected === TabsKey.apr) {
      return data
        .filter((item) => !item.jailed)
        .sort((itemA, itemB) => itemB.apr - itemA.apr);
    }

    return data;
  }, [data, selected]);

  const handleOnClickTab = useCallback(
    (type: TabsKey) => {
      setSelected(type);
      onSelectRow(undefined);
    },
    [onSelectRow]
  );

  return (
    <>
      <Tabs
        selected={selected}
        options={[TabsKey.power, TabsKey.apr, TabsKey.bond].map((type) => {
          return {
            text: TabsKeyConfig[type].title,
            key: type,
            onClick: () => handleOnClickTab(type),
          };
        })}
      />
      <Table data={[]} columns={columns} enableSorting={false} hideBody />
      {selected === TabsKey.power ? (
        <ValidatorTableByGroup
          data={dataGroup}
          columns={columns}
          onSelect={onSelectRow}
        />
      ) : (
        <Display noPadding>
          <Table
            data={dataBySelected}
            columns={columns}
            hideHeader
            enableSorting={false}
            onSelect={(row) => {
              onSelectRow(row ? data[Number(row)] : undefined);
            }}
          />
        </Display>
      )}
    </>
  );

  // return (

  // );
}

export default ValidatorTable;

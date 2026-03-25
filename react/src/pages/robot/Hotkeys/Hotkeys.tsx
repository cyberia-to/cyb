import { createColumnHelper } from '@tanstack/react-table';
import { Display } from 'src/components';
import Table from 'src/components/Table/Table';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import styles from './Hotkeys.module.scss';

const columnHelper = createColumnHelper();

const data = [
  {
    hotkey: '/',
    page: 'all',
    description: 'Focus commander',
  },
  {
    hotkey: 'tab, enter',
    page: 'all',
    description: 'action bar keyboard navigation',
  },
  {
    hotkey: 'f',
    page: 'graph',
    description: 'Toggle graph fullscreen',
  },
];

function Hotkeys() {
  useAdviserTexts({
    defaultText: 'hotkeys info',
  });
  return (
    <Display noPadding>
      <Table
        enableSorting={false}
        data={data}
        columns={[
          columnHelper.accessor('hotkey', {
            header: 'Hotkey',
            cell: (row) => <code className={styles.hotkey}>{row.getValue()}</code>,
          }),
          columnHelper.accessor('page', {
            header: 'Page',
          }),
          columnHelper.accessor('description', {
            header: 'Description',
          }),
        ]}
      />
    </Display>
  );
}

export default Hotkeys;

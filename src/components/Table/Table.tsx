import {
  useReactTable,
  ColumnDef,
  getCoreRowModel,
  flexRender,
  getSortedRowModel,
  InitialTableState,
  SortingState,
  TableOptions,
} from '@tanstack/react-table';

import { useEffect, useState, useCallback } from 'react';
import cx from 'classnames';
import { sessionStorageKeys } from 'src/constants/sessionStorageKeys';
import styles from './Table.module.scss';
import Loader2 from '../ui/Loader2';
import NoItems from '../ui/noItems';
import Triangle from '../atoms/Triangle/Triangle';
import { tableIDs } from './tableIDs';

const storage = sessionStorage;
const SS_KEY = sessionStorageKeys.tableSorting;

type TableIDs = typeof tableIDs;

function getDataFromStorage(): Record<keyof TableIDs, SortingState> {
  return JSON.parse(storage.getItem(SS_KEY) || '{}');
}

function saveDataToStorage(id: string, sorting: SortingState) {
  storage.setItem(
    SS_KEY,
    JSON.stringify({
      ...getDataFromStorage(),
      [id]: sorting,
    })
  );
}

export type Props<T extends object> = {
  columns: ColumnDef<T>[];
  data: T[];
  isLoading?: boolean;
  onSelect?: (id: string | null) => void;

  // prefer not use
  style?: any;
  id?: keyof TableIDs;

  initialState?: InitialTableState;

  // maybe temporary
  enableSorting?: boolean;
  hideHeader?: boolean;
  hideBody?: boolean;
};

function Table<T extends object>({
  columns,
  data,
  isLoading,
  style,
  onSelect,
  initialState,
  id,
  enableSorting = true,
  hideHeader = false,
  hideBody = false,
}: Props<T>) {
  const [selected, setSelected] = useState<string | null>(null);

  const savedSorting = id && getDataFromStorage()[id];
  const [sorting, setSorting] = useState<SortingState>(
    savedSorting || initialState?.sorting || []
  );

  useEffect(() => {
    if (!id) {
      return;
    }

    saveDataToStorage(id, sorting);
  }, [sorting, id]);

  const handleSortingChange = useCallback(
    (sorting: TableOptions<any>['onSortingChange']) => {
      setSorting(sorting);
    },
    []
  );

  const table = useReactTable({
    // debugTable: true,
    columns,
    data,
    state: {
      rowSelection: {},
      sorting,
    },
    initialState,
    enableSorting,
    // enableRowSelection: true,
    // onRowSelectionChange: (state) => {
    //   console.log(state);
    //   debugger;
    // },

    // enableColumnResizing: true,
    // columnResizeMode: 'onChange',
    onSortingChange: handleSortingChange,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  return (
    <>
      <table
        className={cx(styles.table, {
          [styles.selectable]: !!onSelect,
        })}
        style={style}
      >
        {!hideHeader && (
          <thead className={cx({ [styles.hideHeader]: hideHeader })}>
            {table.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id}>
                {headerGroup.headers.map((header) => {
                  return (
                    <th
                      key={header.id}
                      className={cx({
                        [styles.sortable]: header.column.getCanSort(),
                      })}
                      colSpan={header.colSpan}
                      style={{
                        width: header.column.getSize(),
                      }}
                      onClick={header.column.getToggleSortingHandler()}
                    >
                      {flexRender(
                        header.column.columnDef.header,
                        header.getContext()
                      )}
                      &nbsp;
                      {header.column.getCanSort() && (
                        <Triangle
                          disabled={!header.column.getIsSorted()}
                          direction={
                            header.column.getIsSorted() === 'desc'
                              ? 'down'
                              : 'up'
                          }
                        />
                      )}
                      {/* <div
                      {...{
                        onDoubleClick: () => header.column.resetSize(),
                        onMouseDown: header.getResizeHandler(),
                        onTouchStart: header.getResizeHandler(),
                        className: `${styles.resizer} ${
                          header.column.getIsResizing() ? styles.isResizing : ''
                        }`,
                      }}
                    /> */}
                    </th>
                  );
                })}
              </tr>
            ))}
          </thead>
        )}

        {!hideBody && (
          <tbody>
            {table.getRowModel().rows.map((row) => {
              return (
                <tr
                  key={row.id}
                  data-id={row.id}
                  className={cx({ [styles.rowSelected]: row.id === selected })}
                  onClick={(e) => {
                    // TODO: move to tbody

                    if (!onSelect) {
                      return;
                    }

                    if (
                      ['a', 'button', 'input'].includes(
                        (e.target as any).tagName.toLowerCase()
                      )
                    ) {
                      return;
                    }

                    const id = e.currentTarget.getAttribute('data-id');

                    if (id === selected) {
                      setSelected(null);
                      onSelect(null);
                      return;
                    }

                    setSelected(id);
                    onSelect(id);
                  }}
                >
                  {row.getVisibleCells().map((cell) => {
                    return (
                      <td
                        key={cell.id}
                        style={{
                          width: cell.column.getSize(),
                        }}
                      >
                        {flexRender(
                          cell.column.columnDef.cell,
                          cell.getContext()
                        )}
                      </td>
                    );
                  })}
                </tr>
              );
            })}
          </tbody>
        )}
      </table>

      {hideBody ? null : isLoading ? (
        <Loader2 />
      ) : (
        !data.length && <NoItems text="No data" />
      )}
    </>
  );
}

export default Table;

import { Pane } from '@cybercongress/gravity';
import { createColumnHelper } from '@tanstack/react-table';
import { Link } from 'react-router-dom';
import Table from 'src/components/Table/Table';
import { routes } from 'src/routes';
import { ValueImg } from '../../../components';
import { DestinationRoute } from '../hooks/useSourceRouted';
import { renderTableRows } from './income';

type Props = {
  sourceRouted: DestinationRoute[];
  selectRouteFunc?: (index: string | null) => void;
};

function Outcome({ sourceRouted, selectRouteFunc }: Props) {
  const routerData = sourceRouted.map(({ resource, destination, name }) =>
    renderTableRows({ resource, address: destination, alias: name })
  );

  const columnHelper = createColumnHelper<any>();

  return (
    <div>
      <Pane marginY={30} textAlign="center">
        Route your <Link to={routes.oracle.ask.getLink('free energy')}>free energy</Link> (W) to
        those who deserve it
      </Pane>

      <Table
        onSelect={selectRouteFunc}
        columns={[
          columnHelper.accessor('address', {
            header: 'Receiver',
            cell: (info) => info.getValue(),
          }),
          columnHelper.accessor('alias', {
            header: 'Alias',
          }),
          columnHelper.accessor('milliampere', {
            header: (
              <span
                style={{
                  textTransform: 'capitalize',
                }}
              >
                <ValueImg text="milliampere" />
              </span>
            ),
          }),
          columnHelper.accessor('millivolt', {
            header: (
              <span
                style={{
                  textTransform: 'capitalize',
                }}
              >
                <ValueImg text="millivolt" />
              </span>
            ),
          }),
        ]}
        data={routerData}
      />
    </div>
  );
}

export default Outcome;

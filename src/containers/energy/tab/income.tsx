import { Pane } from '@cybercongress/gravity';
import { createColumnHelper } from '@tanstack/react-table';
import { Link } from 'react-router-dom';
import Table from 'src/components/Table/Table';
import { routes } from 'src/routes';
import { Account, ValueImg } from '../../../components';
import { convertResources, formatNumber } from '../../../utils/utils';
import { DestinationRoute } from '../hooks/useSourceRouted';

export function renderTableRows({
  address,
  alias,
  resource,
}: {
  address: DestinationRoute['source'];
  alias: DestinationRoute['alias'];
  resource: DestinationRoute['resource'];
}) {
  return {
    address: <Account address={address} />,
    alias,
    milliampere: resource.milliampere ? formatNumber(convertResources(resource.milliampere)) : 0,
    millivolt: resource.millivolt ? formatNumber(convertResources(resource.millivolt)) : 0,
  };
}

function Income({ destinationRoutes }: { destinationRoutes: DestinationRoute[] }) {
  const data = destinationRoutes.map(({ source, alias, resource }) =>
    renderTableRows({ address: source, alias, resource })
  );

  const columnHelper = createColumnHelper<any>();

  return (
    <div>
      <Pane marginY={30} textAlign="center">
        These <Link to={routes.oracle.ask.getLink('energy')}>energy</Link> (W) was route to you
      </Pane>

      <Table
        columns={[
          columnHelper.accessor('address', {
            header: 'Transmitter',
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
        data={data}
      />
    </div>
  );
}

export default Income;

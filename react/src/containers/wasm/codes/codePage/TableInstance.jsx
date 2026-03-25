import InstanceRow from './InstanceRow';

function TableInstance({ contracts }) {
  return (
    <table style={{ width: '100%' }}>
      <thead>
        <tr>
          <th scope="col">#</th>
          <th scope="col">Label</th>
          <th scope="col">Contract</th>
          <th scope="col">Creator</th>
          <th scope="col">Admin</th>
          <th scope="col">Executions</th>
        </tr>
      </thead>
      <tbody>
        {contracts.map((address, index) => (
          <InstanceRow position={index + 1} address={address} key={address} />
        ))}
      </tbody>
    </table>
  );
}

export default TableInstance;

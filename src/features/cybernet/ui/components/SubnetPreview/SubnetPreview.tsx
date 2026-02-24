import { Link } from 'react-router-dom';
import { Tooltip } from 'src/components';
import AvatarImgIpfs from 'src/containers/portal/components/avataIpfs/AvataImgIpfs';
import { useCurrentContract, useCybernet } from '../../cybernet.context';
import { cybernetRoutes } from '../../routes';
import SubnetPreview from './SubnetPreview';

function SubnetPreview({ subnetUID, withName }: { subnetUID: string }) {
  const { subnetsQuery } = useCybernet();

  const subnet = subnetsQuery.data?.find((subnet) => subnet.netuid === subnetUID);

  const { name, logo } = subnet?.metadata || {};

  const { contractName } = useCurrentContract();

  return (
    <Link to={cybernetRoutes.subnet.getLink('pussy', contractName, subnetUID)}>
      <Tooltip
        tooltip={name}
        contentStyle={{
          display: 'flex',
          alignItems: 'center',
          gap: '0 7px',
        }}
      >
        {logo && (
          <AvatarImgIpfs
            cidAvatar={logo}
            style={{
              width: 16,
              height: 16,
            }}
          />
        )}

        {withName && name}
      </Tooltip>
    </Link>
  );
}

export function SubnetPreviewGroup({ uids }: { uids: string[] }) {
  return (
    <div
      style={{
        display: 'flex',
      }}
    >
      {uids.map((uid) => (
        <SubnetPreview key={uid} subnetUID={uid} />
      ))}
    </div>
  );
}

export default SubnetPreview;

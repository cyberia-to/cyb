import { useEffect } from 'react';
import { useSelector } from 'react-redux';
import { Display } from 'src/components';
import { useAdviser } from 'src/features/adviser/context';
import { useRobotContext } from 'src/pages/robot/robot.context';
import { RootState } from 'src/redux/store';
import { useGetCommunity } from '../../hooks';
import styles from './Follows.module.scss';
import CommunityEntity from './ui/CommunityEntity';

function Follows() {
  const { address, isOwner } = useRobotContext();
  const mainAccountCommunity = useSelector((state: RootState) => state.backend.community);

  const communityHook = useGetCommunity(address, { skip: isOwner });

  const { setAdviser } = useAdviser();

  useEffect(() => {
    setAdviser(
      <>
        neurons surrounding <br /> trace the connections
      </>
    );
  }, [setAdviser]);

  const community = isOwner ? mainAccountCommunity : communityHook.community;
  const loading = isOwner
    ? {
        friends: false,
        following: false,
        followers: false,
      }
    : communityHook.loading;
  return (
    <Display noPadding color="blue">
      <div className={styles.wrapper}>
        <CommunityEntity
          title="Friends"
          loading={loading.friends}
          noItemsTitle="No Friends"
          items={community.friends}
        />
        <CommunityEntity
          title="Following"
          loading={loading.following}
          noItemsTitle="No Following"
          items={community.following}
        />
        <CommunityEntity
          title="Followers"
          loading={loading.followers}
          noItemsTitle="No Followers"
          items={community.followers}
        />
      </div>
    </Display>
  );
}

export default Follows;

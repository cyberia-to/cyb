import { useQuery } from '@tanstack/react-query';
import cx from 'classnames';
import { useState } from 'react';
import useQueueIpfsContent from 'src/hooks/useQueueIpfsContent';
import styles from './styles.module.scss';

const getRoboHashImage = (addressCyber: string) => `https://robohash.org/${addressCyber}`;

type Props = {
  cidAvatar?: string;
  img?: string;
  addressCyber?: string;
  className?: string;
};

function AvatarImgIpfs({ img = '', cidAvatar, addressCyber, className, ...props }: Props) {
  const { fetchWithDetails } = useQueueIpfsContent();
  const [imgError, setImgError] = useState(false);

  const { data: avatar, isLoading } = useQuery(
    ['getAvatar', cidAvatar],
    async () => {
      const details = await fetchWithDetails!(cidAvatar!, 'image');
      return details?.content || null;
    },
    {
      enabled: Boolean(fetchWithDetails && cidAvatar),
    }
  );

  const avatarImage =
    avatar || (addressCyber && getRoboHashImage(addressCyber)) || img || getRoboHashImage('null');

  // Show placeholder while loading IPFS avatar or if image failed to load
  if ((cidAvatar && isLoading) || imgError) {
    return (
      <div
        className={cx(styles.imgAvatar, styles.placeholder, className)}
        title={cidAvatar || addressCyber || ''}
      />
    );
  }

  return (
    <img
      {...props}
      src={avatarImage}
      className={cx(styles.imgAvatar, className)}
      alt=""
      onError={() => setImgError(true)}
    />
  );
}

export default AvatarImgIpfs;

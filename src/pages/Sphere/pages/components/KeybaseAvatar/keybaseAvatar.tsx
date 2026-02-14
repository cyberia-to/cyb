import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import { keybaseAvatar } from 'src/utils/search/utils';

const img = require('images/logo-cyb-v3.svg');

function KeybaseAvatar({
  identity,
  size = '80px',
}: {
  identity?: string;
  size?: string;
}) {
  const { data } = useQuery({
    queryKey: ['KeybaseAvatar', identity],
    queryFn: () => keybaseAvatar(identity).catch((e) => {}),
    enabled: Boolean(identity),
  });

  const imgKeyBase = useMemo(() => {
    if (data?.them && data.them[0]?.pictures) {
      return data.them[0].pictures.primary.url;
    }
    return undefined;
  }, [data]);

  return (
    <img
      style={{
        width: size,
        height: size,
        borderRadius: imgKeyBase ? '50%' : 'none',
      }}
      alt="img-keybase-avatar"
      src={imgKeyBase || img}
    />
  );
}

export default KeybaseAvatar;

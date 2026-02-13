import { formatCurrency } from 'src/utils/utils';
import { PREFIXES } from 'src/containers/ipfs/components/metaInfo';
import useParticle from '../../../hooks/useParticle';
import styles from './ParticleSize.module.scss';

// for future use if no UI needed
// eslint-disable-next-line import/no-unused-modules
export function useParticleSize(cid: string) {
  const { content, details, status } = useParticle(cid);

  const size = content?.meta?.size || details?.content?.length;

  return {
    size,
    isLoading: status === 'pending' || status === 'executing',
  };
}

function ParticleSize({ cid }: { cid: string }) {
  const { isLoading, size } = useParticleSize(cid);

  let content;

  if (isLoading) {
    // use Loading component
    content = 'loading...';
  } else if (size) {
    content = formatCurrency(size, 'B', 0, PREFIXES);
  } else {
    content = 'unknown';
  }

  return <span className={styles.size}>🟥 {content}</span>;
}

export default ParticleSize;

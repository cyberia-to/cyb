export function isParticle(value: string) {
  // copied from src/utils/config.ts , to prevent crash in worker, need refactor
  // import { PATTERN_IPFS_HASH } from 'src/utils/config';
  return Boolean(value.match(/^Qm[a-zA-Z0-9]{44}$/g));
}

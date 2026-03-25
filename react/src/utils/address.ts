import { PATTERN_CYBER_CONTRACT } from 'src/constants/patterns';
import { Networks } from 'src/types/networks';

export function getTypeFromAddress(address: string): Networks {
  if (address.startsWith('bostrom')) {
    return Networks.BOSTROM;
  } else if (address.startsWith('osmo')) {
    return Networks.OSMO;
  } else if (address.startsWith('0x')) {
    return Networks.ETH;
  } else if (address.startsWith('terra')) {
    return Networks.TERRA;
  } else if (address.startsWith('cyber')) {
    return Networks.CYBER;
  } else if (address.startsWith('cosmos')) {
    return Networks.COSMOS;
  } else if (address.startsWith('pussy')) {
    return Networks.SPACE_PUSSY;
  }

  return undefined;
}

export function isPussyAddress(address: string): boolean {
  // fix
  return address.match(PATTERN_CYBER_CONTRACT);
}

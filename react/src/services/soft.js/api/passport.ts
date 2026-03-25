// TODO: add type

import { CyberClient } from '@cybercongress/cyber-js';
// import { CONTRACT_ADDRESS_PASSPORT } from 'src/containers/portal/utils';
import { getPassport } from 'src/services/passports/lcd.ts';

// https://github.com/cybercongress/cw-cybergift/tree/main/contracts/cw-cyber-passport/schema
export type PassportContractQuery =
  | {
      active_passport: {
        address: string;
      };
    }
  | {
      passport_by_nickname: {
        nickname: string;
      };
    }
  | {
      nft_info: {
        token_id: string;
      };
    };

export function queryPassportContract(query: PassportContractQuery, _queryClient: CyberClient) {
  return getPassport(query);
  // return queryClient.queryContractSmart(CONTRACT_ADDRESS_PASSPORT, query);
}

import { toAscii, toBase64 } from '@cosmjs/encoding';
import axios from 'axios';
import defaultNetworks from 'src/constants/defaultNetworks';
import { CONTRACT_ADDRESS_PASSPORT } from 'src/containers/portal/utils';
import { PassportContractQuery } from 'src/services/soft.js/api/passport';

// need this request to query passports with any queryClient chain
// eslint-disable-next-line import/prefer-default-export
export async function getPassport(query: PassportContractQuery) {
  try {
    const response = await axios.get(
      `${
        defaultNetworks.bostrom.LCD_URL
      }/cosmwasm/wasm/v1/contract/${CONTRACT_ADDRESS_PASSPORT}/smart/${toBase64(
        toAscii(JSON.stringify(query))
      )}`
    );
    return response.data.data;
  } catch (error) {
    // Re-throw with the contract error message so redux can match known errors
    if (axios.isAxiosError(error) && error.response?.data?.message) {
      throw new Error(error.response.data.message);
    }
    throw error;
  }
}

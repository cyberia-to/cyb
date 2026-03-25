import axios from 'axios';
import { LCD_URL } from 'src/constants/config';
import { dataOrNull } from 'src/utils/axios';

export const getTxs = async (txHash: string) => {
  const response = await axios({
    method: 'get',
    url: `${LCD_URL}/cosmos/tx/v1beta1/txs/${txHash}`,
  });
  return dataOrNull(response);
};

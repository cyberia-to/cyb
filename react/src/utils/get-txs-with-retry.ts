import { requestWithRetry } from './request-with-retry';

/* eslint-disable import/prefer-default-export */
export const getTxsWithRetry = async (txs: string) => {
  try {
    const response = await requestWithRetry({
      method: 'get',
      url: `${LCD_URL}/cosmos/tx/v1beta1/txs/${txs}`,
    });

    return response.data;
  } catch (e) {
    console.error(e);
    return null;
  }
};

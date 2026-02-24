/* eslint-disable import/prefer-default-export */
import { AxiosResponse } from 'axios';

export function dataOrNull<T>(response: AxiosResponse<T>) {
  try {
    return response.data;
  } catch (_e) {
    return null;
  }
}

/* eslint-disable no-promise-executor-return */
/* eslint-disable no-await-in-loop */
/* eslint-disable import/prefer-default-export */
import axios, { AxiosError, AxiosRequestConfig, AxiosResponse } from 'axios';

export const requestWithRetry = async (
  config: AxiosRequestConfig,
  interval = 1500,
  timeout = 10000
): Promise<AxiosResponse> => {
  const startTime = Date.now();

  while (Date.now() - startTime < timeout) {
    try {
      const response = await axios(config);
      return response;
    } catch (error) {
      const axiosError = error as AxiosError;

      if (
        axiosError.response?.status !== 404 ||
        Date.now() - startTime >= timeout
      ) {
        throw axiosError;
      }

      await new Promise((resolve) => setTimeout(resolve, interval));
    }
  }

  throw new Error(`Request failed after ${timeout} ms`);
};

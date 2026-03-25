// eslint-disable-next-line import/prefer-default-export
export const setBandwidth = (remained, maxValue) => {
  return {
    type: 'SET_BANDWIDTH',
    remained,
    maxValue,
  };
};

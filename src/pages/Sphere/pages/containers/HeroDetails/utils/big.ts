import BigNumber from 'bignumber.js';

function big(value: BigNumber.Value) {
  return new BigNumber(value);
}

export default big;

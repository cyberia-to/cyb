function getPrefixNumber(power: number, value: number) {
  if (!power || !value) {
    return 0;
  }

  return Math.floor(Math.log(value) / Math.log(power));
}

export default getPrefixNumber;

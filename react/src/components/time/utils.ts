const unixTimestamp = (secondsTime: number) => {
  const years = Math.floor(secondsTime / 31536000);
  const months = Math.floor(secondsTime / 2592000);
  const days = Math.floor(secondsTime / 86400);
  const hours = Math.floor(((secondsTime % 31536000) % 86400) / 3600);
  const minutes = Math.floor((((secondsTime % 31536000) % 86400) % 3600) / 60);
  const seconds = Math.floor((((secondsTime % 31536000) % 86400) % 3600) % 60);

  return {
    years,
    months,
    days,
    hours,
    minutes,
    seconds,
  };
};

export default unixTimestamp;

import dateFormat from 'dateformat';

export const numberToUtcDate = (timestamp: number) =>
  dateFormat(new Date(timestamp), 'yyyy-mm-dd"T"HH:MM:ss.l', true);

export const dateToUtcNumber = (isoString: string) =>
  Date.parse(isoString.endsWith('Z') ? isoString : `${isoString}Z`);

export const getNowUtcNumber = () => Date.now();

function roundMilliseconds(dateTimeString: string) {
  const date = new Date(dateTimeString);
  const roundedMilliseconds = Math.round(date.getMilliseconds() / 1000) * 1000;
  date.setMilliseconds(roundedMilliseconds);
  return dateFormat(date, 'yyyy-mm-dd"T"HH:MM:ss.l');
}

function pluralizeUnit(quantity: number, unit: string): string {
  return quantity === 1 ? unit : `${unit}s`;
}

const minuteInMs = 60000; // 60 seconds * 1000 milliseconds
const hourInMs = 3600000; // 60 minutes * 60 seconds * 1000 milliseconds
const dayInMs = 86400000; // 24 hours * 60 minutes * 60 seconds * 1000 milliseconds

function convertTimestampToString(timestamp: number): string {
  if (timestamp < minuteInMs) {
    const seconds = Math.floor(timestamp / 1000);
    return `${seconds} ${pluralizeUnit(seconds, 'second')}`;
  }
  if (timestamp < hourInMs) {
    const minutes = Math.floor(timestamp / minuteInMs);
    return `${minutes} ${pluralizeUnit(minutes, 'minute')}`;
  }
  if (timestamp < dayInMs) {
    const hours = Math.floor(timestamp / hourInMs);
    return `${hours} ${pluralizeUnit(hours, 'hour')}`;
  }

  const days = Math.floor(timestamp / dayInMs);
  return `${days} ${pluralizeUnit(days, 'day')}`;
}

export { roundMilliseconds, convertTimestampToString };

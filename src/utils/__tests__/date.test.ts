import { dateToUtcNumber, numberToUtcDate, roundMilliseconds } from '../date';

test('numberToDate should convert timestamp to formatted date string', () => {
  const timestamp = 0;
  const result = numberToUtcDate(timestamp);
  expect(result).toBe('1970-01-01T00:00:00.000');
});

test('dateToUtcNumber should convert ISO string to UTC timestamp regardless of "Z"', () => {
  const isoString = '1970-01-01T00:00:00.000';
  // const isoStringZ = '1970-01-01T00:00Z';
  expect(dateToUtcNumber(isoString)).toBe(0);
  expect(numberToUtcDate(0)).toBe(isoString);
});

test('dateToUtcNumber should convert ISO string to timestamp', () => {
  const isoString = '2000-01-01T12:00:00.000';
  const result = dateToUtcNumber(isoString);
  expect(result).toBe(946728000000);
});

test('roundMilliseconds should round the milliseconds of a date-time string', () => {
  const dateTimeString = '2022-01-01T12:00:00.123';
  const result = roundMilliseconds(dateTimeString);
  expect(result).toBe('2022-01-01T12:00:00.000');
});

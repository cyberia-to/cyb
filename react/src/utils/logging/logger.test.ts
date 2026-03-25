import cyblog from './cyblog';

describe('createCybLog', () => {
  beforeEach(() => {
    cyblog.clear();
  });

  it('should append log items to the log list', () => {
    cyblog.info('Test log message');
    cyblog.error('Test error message');

    expect(cyblog.logList.length).toBe(2);
    expect(cyblog.logList[0].level).toBe('info');
    expect(cyblog.logList[0].message).toBe('Test log message');
    expect(cyblog.logList[1].level).toBe('error');
    expect(cyblog.logList[1].message).toBe('Test error message');
  });

  it('should truncate log list when it exceeds 1000 items', () => {
    for (let i = 0; i < 1100; i++) {
      cyblog.info(`Log message ${i}`);
    }

    expect(cyblog.logList.length).toBe(1000);
    expect(cyblog.logList[0].message).toBe('Log message 100');
    expect(cyblog.logList[999].message).toBe('Log message 1099');
  });

  it('should log messages to the console', () => {
    jest.spyOn(Storage.prototype, 'getItem').mockImplementation(() => 'true');
    const consoleLogInfoSpy = jest.spyOn(console, 'info');
    const consoleLogErrorSpy = jest.spyOn(console, 'error');

    cyblog.info('Test log message');
    cyblog.error('Test error message');

    expect(consoleLogInfoSpy).toHaveBeenCalledTimes(1);
    expect(consoleLogErrorSpy).toHaveBeenCalledTimes(1);
    expect(consoleLogInfoSpy.mock.calls[0][0]).toBe('Test log message');
    expect(consoleLogErrorSpy.mock.calls[0][0]).toBe('Test error message');
  });

  it('should format log messages using the provided formatter', () => {
    const formatter = (message) => `[CUSTOM] ${message}`;

    cyblog.info('Test log message', { formatter });

    expect(cyblog.logList[0].message).toBe('[CUSTOM] Test log message');
  });

  it('should exclude formatter and stacktrace from the log entry context', () => {
    const formatter = (message) => `[CUSTOM] ${message}`;
    const stacktrace = 'Error stacktrace';

    cyblog.info('Test log message', { formatter, stacktrace });

    expect(cyblog.logList[0].context).toEqual({});
  });
});

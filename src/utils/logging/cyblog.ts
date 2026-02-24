import _, { isEmpty } from 'lodash';
import { CYBLOG_BROADCAST_CHANNEL_NAME } from './constants';
import { ConsoleLogParams, LogContext, LogItem, LogLevel } from './types';

const logList: LogItem[] = [];

function createCybLog<T>(defaultContext: Partial<LogContext<T>> = {}) {
  function appendLog(logItem: LogItem, truncate = true) {
    logList.push(logItem);

    while (truncate && logList.length > 1000) {
      logList.shift(); // Remove the first element to keep the list size <= 1000
    }
  }
  let consoleLogParams = {} as ConsoleLogParams;

  const channel = new BroadcastChannel(CYBLOG_BROADCAST_CHANNEL_NAME);

  channel.onmessage = (event) => {
    if (event.data.type === 'params') {
      consoleLogParams = { ...consoleLogParams, ...event.data.value };
    }
  };

  const getConsoleLogParams = () => consoleLogParams;

  function consoleLog<T>(level: LogLevel, message: T, context: Partial<LogContext<T>>) {
    const ctx = _.omit(context, ['formatter', 'thread', 'module', 'unit', 'data']);
    const { thread = '', module = '', unit = '', data = '' } = context;
    const ctxItem = isEmpty(ctx) ? '' : ctx;

    if (Array.isArray(message)) {
      console[level](...message, ctxItem);
      return;
    }

    if (context?.formatter) {
      console[level](context?.formatter(message), ctxItem);
      return;
    }

    console[level](`[${thread}:${module}:${unit}] ${message}`, data, ctxItem);
  }

  // eslint-disable-next-line import/no-unused-modules
  function log<T>(level: LogLevel, message: string | T, context: LogContext<any> = defaultContext) {
    try {
      const formattedMessage = context?.formatter ? context?.formatter(message) : message;

      const logEntry = {
        timestamp: new Date(),
        level,
        message: formattedMessage,
        stacktrace: context?.stacktrace,
        context: _.omit(context, ['formatter', 'stacktrace']),
      };

      appendLog(logEntry);
      // !!localStorage.getItem(LOCAL_STORAGE_USE_CONSOLE_LOG_KEY) &&
      const showConsoleLog = Object.keys(consoleLogParams).reduce((acc: boolean, key: string) => {
        const params = consoleLogParams[key];
        const contextItem = context[key];
        if (params && contextItem) {
          return (
            acc || params === 'all' || params.length === 0 || params.some((p) => p === contextItem)
          );
        }
        return acc;
      }, false);

      if (showConsoleLog) {
        consoleLog(level, message, context);
      }
    } catch (error) {
      console.log('cyblog error', error);
    }
  }

  function info<T>(message: T, context?: LogContext<string | T>) {
    return log('info', message, context);
  }

  function error<T>(message: T, context?: LogContext<string | T>) {
    return log('error', message, context);
  }

  function warn<T>(message: T, context?: LogContext<string | T>) {
    return log('warn', message, context);
  }

  function trace<T>(message: T, context?: LogContext<string | T>) {
    return log('warn', message, context);
  }

  function normalizeLog() {
    return logList.map((logItem) => {
      const { context, ...rest } = logItem;
      const {
        unit = '',
        module = '',
        thread = '',
        data = '',
        error = '',
        stacktrace = '',
      } = context || {};
      return {
        ...rest,
        unit,
        module,
        thread,
        data, //: JSON.stringify(data),
        error,
        stacktrace,
      };
    });
  }

  return {
    log,
    info,
    error,
    warn,
    trace,
    logList,
    getLogs: () => normalizeLog(),
    clear: () => logList.splice(0, logList.length),
    getConsoleLogParams,
  };
}

export const createCyblogChannel = (defaultContext: Partial<LogContext<T>> = {}) => {
  const channel = new BroadcastChannel(CYBLOG_BROADCAST_CHANNEL_NAME);

  function postLogToChannel<T>(level: LogLevel, message: T, context?: LogContext<string | T>) {
    const ctx = { ...defaultContext, ...context };
    if (context?.error) {
      ctx.error = JSON.stringify(context.error);
    }
    channel.postMessage({
      type: 'log',
      value: { level, message, context: ctx },
    });
  }

  function info<T>(message: T, context?: LogContext<string | T>) {
    return postLogToChannel('info', message, context);
  }

  function error<T>(message: T, context?: LogContext<string | T>) {
    return postLogToChannel('error', message, context);
  }

  function warn<T>(message: T, context?: LogContext<string | T>) {
    return postLogToChannel('warn', message, context);
  }

  function trace<T>(message: T, context?: LogContext<string | T>) {
    return postLogToChannel('warn', message, context);
  }

  return { info, error, warn, trace };
};

const cyblog = createCybLog({ thread: 'main' });

export type LogFunc = (message: T, context?: LogContext<string | T>) => void;

export type CyblogChannel = ReturnType<typeof createCyblogChannel>;

export default cyblog;

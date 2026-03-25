export type LogLevel = 'info' | 'warn' | 'error' | 'trace';

export type LogThreads = 'main' | 'cozo' | 'bckd';

export type LogContext<T> = {
  thread?: LogThreads;
  module?: string;
  unit?: string;
  data?: any;
  stacktrace?: any[];
  error?: Error | any;
  formatter?: (message: T) => string;
};

export type LogItem = {
  timestamp: Date;
  level: LogLevel;
  message: any;
  context?: Omit<LogContext<any>, 'formatter'>; //'stacktrace' |
  stacktrace?: any[];
};

export type ConsoleLogParams = {
  thread?: LogThreads[] | 'all';
  module?: string[] | 'all';
  unit?: string[] | 'all';
};

export type LogContextParams<T> = LogContext<string | T> | any[];

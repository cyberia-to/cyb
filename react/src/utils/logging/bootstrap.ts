import {
  CYBLOG_BROADCAST_CHANNEL_NAME,
  CYBLOG_CONSOLE_PARAMS_DEFAULT,
  CYBLOG_LOG_SHOW,
} from './constants';
import { ConsoleLogParams } from './types';

let isLogging = false;

const originalConsoleError = console.error;

// Override the console.error function
console.error = (...args) => {
  // Check if the current call is made by your custom logger
  if (isLogging) {
    // It's a call from your logger; directly call the original console.error to avoid loop
    originalConsoleError.apply(console, args);
  } else {
    // Mark as logging to avoid re-entrance
    isLogging = true;

    // Call your custom logger here
    // For demonstration, let's assume your logger has a method named logError
    cyblog.error(...args);

    // Optionally, call the original console.error function if you still want the error to be logged in the console
    originalConsoleError.apply(console, args);

    // Reset the flag
    isLogging = false;
  }
};

declare global {
  interface Window {
    setCyblogConsole: (val: boolean | ConsoleLogParams) => void;
  }
}

const channel = new BroadcastChannel(CYBLOG_BROADCAST_CHANNEL_NAME);

const updateCyblogConsoleLogParams = (params: ConsoleLogParams) =>
  channel.postMessage({ type: 'params', value: params });

window.setCyblogConsole = (val: boolean | ConsoleLogParams) => {
  const params = (
    typeof val === 'boolean' ? (val ? CYBLOG_CONSOLE_PARAMS_DEFAULT : {}) : val
  ) as ConsoleLogParams;
  updateCyblogConsoleLogParams(params);
};

const showCyblogConsoleLogMessage = (params) => {
  // const params = cyblog.getConsoleLogParams();
  const keys = Object.keys(params);
  if (keys.length === 0) {
    console.log('📺 cyblog.console disabled.');
    return;
  }
  const msgs = keys.map((k) => {
    return params[k] === 'all' ? `${k}=all` : `${k}=[${params[k].join(' ,')}]`;
  });
  console.log(`📺 cyblog.console on: ${msgs.join(',')}`);
};

export const initCyblog = () => {
  const channel = new BroadcastChannel(CYBLOG_BROADCAST_CHANNEL_NAME);

  channel.onmessage = (event) => {
    const { type: logType, value } = event.data;
    if (logType === 'log') {
      const { level, message, context } = value;
      cyblog[level](message, context);
    } else if (logType === 'params') {
      showCyblogConsoleLogMessage(value);
      localStorage.setItem(CYBLOG_LOG_SHOW, JSON.stringify(value));
    }
  };

  const params = JSON.parse(localStorage.getItem(CYBLOG_LOG_SHOW) || '{}');
  updateCyblogConsoleLogParams(params);

  showCyblogConsoleLogMessage(cyblog.getConsoleLogParams());
};

import { useState } from 'react';

function useLog(): [
  string[],
  (messages: string[] | string) => void,
  (message: string) => void,
  () => void,
] {
  const [logMessages, setLogMessages] = useState<string[]>([]);

  const appendLog = (messages: string[] | string) => {
    const items = Array.isArray(messages) ? messages : [messages];
    setLogMessages((prev) => [...prev, ...items]);
  };

  const updateLastLog = (message: string) =>
    setLogMessages((prev) => [...prev.slice(0, prev.length - 1), message]);

  const clearLog = () => setLogMessages([]);

  return [logMessages, appendLog, updateLastLog, clearLog];
}

export default useLog;

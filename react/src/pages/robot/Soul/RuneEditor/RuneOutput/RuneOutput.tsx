import { forwardRef, useCallback, useImperativeHandle, useMemo, useRef, useState } from 'react';
import styles from './RuneOutput.module.scss';

type RuneOutputProps = {
  linesCount?: number;
};

export type RuneOutputHandle = {
  put: (items: string[], prefix?: string) => void;
  clear: () => void;
  scrollIntoView: () => void;
};

const RuneOutput = forwardRef<RuneOutputHandle, RuneOutputProps>(({ linesCount = 18 }, ref) => {
  const outputRef = useRef<HTMLTextAreaElement | null>(null);

  const [log, setLog] = useState<string[]>([]);

  const putToLog = useCallback((lines: string[], prefix = '') => {
    setLog((log) => [...log, ...lines.map((l) => `${prefix}${l}`)]);
  }, []);

  const logText = useMemo(() => log.join('\r\n'), [log]);

  const clearLog = () => setLog([]);

  useImperativeHandle(ref, () => ({
    put: putToLog,
    clear: clearLog,
    scrollIntoView: () => outputRef.current?.scrollIntoView({ behavior: 'smooth' }),
  }));

  return <textarea ref={outputRef} value={logText} className={styles.logArea} rows={linesCount} />;
});

RuneOutput.displayName = 'RuneOutput';

export default RuneOutput;

export const isAbortException = (e: Error) => e instanceof DOMException && e.name === 'AbortError';

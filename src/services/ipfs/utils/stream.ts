/* eslint-disable valid-jsdoc */
/* eslint-disable import/no-unused-modules */
import { fileTypeFromBuffer } from 'file-type';
import { concat as uint8ArrayConcat } from 'uint8arrays/concat';
import { Uint8ArrayLike } from '../types';

type ResultWithMime = {
  result: Uint8ArrayLike;
  mime: string | undefined;
  firstChunk: Uint8Array | undefined;
};

type StreamDoneCallback = (
  chunks: Array<Uint8Array>,
  mime: string | undefined
) => Promise<void> | void;

// interface AsyncIterableWithReturn<T> extends AsyncIterable<T> {
//   return?: (value?: unknown) => Promise<IteratorResult<T>>;
// }

export const getMimeFromUint8Array = async (
  raw: Uint8Array | undefined
): Promise<string | undefined> => {
  if (!raw) {
    return 'unknown';
  }
  // TODO: try to pass only first N-bytes
  const fileType = await fileTypeFromBuffer(raw);

  return fileType?.mime || 'text/plain';
};

export async function toAsyncIterableWithMime(
  stream: ReadableStream<Uint8Array>,
  flush?: StreamDoneCallback
): Promise<ResultWithMime> {
  const [firstChunkStream, fullStream] = stream.tee();
  const chunks: Array<Uint8Array> = []; // accumulate all the data to pim/save

  // Read the first chunk from the stream
  const firstReader = firstChunkStream.getReader();
  const { value } = await firstReader.read();
  const mime = value ? await getMimeFromUint8Array(value) : undefined;

  const restReader = fullStream.getReader();

  const asyncIterable: AsyncIterable<Uint8Array> = {
    async *[Symbol.asyncIterator]() {
      while (true) {
        const { done, value } = await restReader.read();
        if (done) {
          flush?.(chunks, mime);
          return; // Exit the loop when done
        }
        flush && chunks.push(value);
        yield value; // Yield the value to the consumer
      }
    },
  };

  return { mime, result: asyncIterable, firstChunk: value };
}

export async function toReadableStreamWithMime(
  stream: ReadableStream<Uint8Array>,
  flush?: StreamDoneCallback
): Promise<ResultWithMime> {
  const [firstChunkStream, fullStream] = stream.tee();
  const chunks: Array<Uint8Array> = []; // accumulate all the data to pim/save

  // Read the first chunk from the stream
  const firstReader = firstChunkStream.getReader();
  const { value } = await firstReader.read();
  const mime = value ? await getMimeFromUint8Array(value) : undefined;

  const modifiedStream = new ReadableStream<Uint8Array>({
    async pull(controller) {
      const restReader = fullStream.getReader();
      const { done, value } = await restReader.read();
      if (done) {
        controller.close();
        flush?.(chunks, mime);
      } else {
        controller.enqueue(value);
        flush && chunks.push(value);
      }
      restReader.releaseLock();
    },
    cancel() {
      firstChunkStream.cancel();
      fullStream.cancel();
    },
  });

  return { mime, result: modifiedStream, firstChunk: value };
}

export type onProgressCallback = (progress: number) => void;

export class StreamDrainTimeoutError extends Error {
  constructor(timeoutMs: number) {
    super(`stream drain timed out after ${timeoutMs}ms`);
    this.name = 'StreamDrainTimeoutError';
  }
}

// Default wall-clock budget for draining a content stream into a Uint8Array.
// Must be larger than any per-source fetch timeout in QueueManager so that a
// slow-but-progressing download isn't killed prematurely; short enough that a
// peer that delivers a prefix and stalls doesn't hang the UI forever.
export const STREAM_DRAIN_TIMEOUT_MS = 30_000;

export const getResponseResult = async (
  response: Uint8ArrayLike,
  onProgress?: onProgressCallback,
  timeoutMs: number = STREAM_DRAIN_TIMEOUT_MS
): Promise<Uint8Array | undefined> => {
  if (response instanceof Uint8Array) {
    onProgress?.(response.byteLength);
    return response;
  }

  let bytesDownloaded = 0;
  const chunks: Array<Uint8Array> = [];

  const timeoutController = new AbortController();
  const timeoutId = setTimeout(() => timeoutController.abort(), timeoutMs);

  const drain = async (): Promise<Uint8Array | undefined> => {
    if (response instanceof ReadableStream) {
      const reader = response.getReader();
      timeoutController.signal.addEventListener('abort', () => reader.cancel().catch(() => {}));
      try {
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          if (value) {
            chunks.push(value);
            bytesDownloaded += value.byteLength;
            onProgress?.(bytesDownloaded);
          }
        }
      } finally {
        reader.releaseLock();
      }
      return uint8ArrayConcat(chunks);
    }

    if (Symbol.asyncIterator in response) {
      const iterator = response[Symbol.asyncIterator]();
      timeoutController.signal.addEventListener('abort', () => {
        // Best-effort close; async iterators may not expose return().
        iterator.return?.(undefined)?.catch(() => {});
      });
      for await (const chunk of { [Symbol.asyncIterator]: () => iterator }) {
        if (chunk instanceof Uint8Array) {
          chunks.push(chunk);
          bytesDownloaded += chunk.byteLength;
          onProgress?.(bytesDownloaded);
        }
      }
      return uint8ArrayConcat(chunks);
    }

    return undefined;
  };

  try {
    return await drain();
  } catch (error) {
    if (timeoutController.signal.aborted) {
      throw new StreamDrainTimeoutError(timeoutMs);
    }
    throw error;
  } finally {
    clearTimeout(timeoutId);
  }
};

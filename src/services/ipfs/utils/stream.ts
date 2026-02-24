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

export const getResponseResult = async (
  response: Uint8ArrayLike,
  onProgress?: onProgressCallback
) => {
  let bytesDownloaded = 0;
  try {
    if (response instanceof Uint8Array) {
      onProgress?.(response.byteLength);
      return response;
    }
    const chunks: Array<Uint8Array> = [];

    if (response instanceof ReadableStream) {
      const reader = response.getReader();

      const readStream = async ({
        done,
        value,
      }: ReadableStreamReadResult<Uint8Array>): Promise<Uint8Array> => {
        if (done) {
          return uint8ArrayConcat(chunks);
        }

        chunks.push(value!);
        bytesDownloaded += value!.byteLength;
        onProgress?.(bytesDownloaded);
        return reader.read().then(readStream);
      };

      const readArray: Uint8Array = await reader.read().then(readStream);

      return readArray;
    }

    if (Symbol.asyncIterator in response) {
      const reader = response[Symbol.asyncIterator]();

      // if (cid === 'QmRqms6Utkk6L4mtyLQXY2spcQ8Pk7fBBTNjvxa9jTNrXp') {
      //   debugger;
      // }
      // eslint-disable-next-line no-restricted-syntax
      for await (const chunk of reader) {
        if (chunk instanceof Uint8Array) {
          chunks.push(chunk);
          bytesDownloaded += chunk.byteLength;
          onProgress?.(bytesDownloaded);
        }
      }
      const result = uint8ArrayConcat(chunks);
      return result;
    }
    return undefined;
  } catch (error) {
    console.error(`Error reading stream/iterable.\r\n Probably Hot reload error!`, error);

    // throw error;

    return undefined;
  }
};

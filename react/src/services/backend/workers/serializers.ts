import type { TransferHandler } from 'comlink';
import { IPFSContent } from 'src/services/ipfs/types';

export type IPFSContentTransferable = Omit<IPFSContent, 'result'> & {
  port: MessagePort;
};

function createAsyncIterable(port: MessagePort): AsyncIterable<Uint8Array> {
  return {
    async *[Symbol.asyncIterator](): AsyncGenerator<Uint8Array, void, undefined> {
      let done = false;
      while (!done) {
        // eslint-disable-next-line no-loop-func
        const promise = new Promise<Uint8Array | null>((resolve) => {
          // resolve = res;
          port.onmessage = (event: MessageEvent) => {
            if (event.data === null) {
              done = true;
              resolve(null);
            } else {
              resolve(event.data);
            }
          };
        });
        // eslint-disable-next-line no-await-in-loop
        const value = await promise;
        // eslint-disable-next-line no-await-in-loop
        if (value !== null) {
          yield value;
        }
      }
    },
  };
}

const IPFSContentTransferHandler: TransferHandler<
  IPFSContent | undefined,
  IPFSContentTransferable | null
> = {
  canHandle: (obj: IPFSContent | undefined) =>
    obj?.result && typeof obj.result[Symbol.asyncIterator] === 'function',
  serialize(obj: IPFSContent) {
    if (obj === undefined) {
      return [null, []];
    }
    const { result, ...rest } = obj;
    const { port1, port2 } = new MessageChannel();
    if (result) {
      (async () => {
        // eslint-disable-next-line no-restricted-syntax
        for await (const value of result) {
          port1.postMessage(value);
        }
        port1.postMessage(null); // Send  "end" message

        port1.close();
      })();
    }
    return [{ ...rest, port: port2 }, [port2]];
  },
  deserialize(serializedObj: IPFSContentTransferable | null) {
    if (!serializedObj) {
      return undefined;
    }
    const { port, ...rest } = serializedObj;

    return {
      ...rest,
      result: createAsyncIterable(port),
    };
  },
};

export {
  IPFSContentTransferHandler,
  // serializeIPFSContent,
  // deserializeIPFSContent,
};

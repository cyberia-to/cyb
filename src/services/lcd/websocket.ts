import { Observable } from 'rxjs';
import { WEBSOCKET_URL } from 'src/constants/config';
import { NeuronAddress } from 'src/types/base';
import { LogFunc } from 'src/utils/logging/cyblog';

export const getIncomingTransfersQuery = (address: NeuronAddress) =>
  `tm.event='Tx' AND transfer.recipient='${address}'`;

// eslint-disable-next-line import/no-unused-modules
export function createNodeWebsocketObservable(address: NeuronAddress, query: string, log: LogFunc) {
  return new Observable((subscriber) => {
    const ws = new WebSocket(WEBSOCKET_URL);

    ws.onopen = () => {
      log(`node ws connected to ${WEBSOCKET_URL} with ${query}`);
      ws.send(
        JSON.stringify({
          jsonrpc: '2.0',
          method: 'subscribe',
          id: '0',
          params: { query },
        })
      );
    };

    ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      log(`node ws ${address} onmessage`, message);
      subscriber.next(message.result);
    };

    ws.onerror = (event) => {
      log(`node ws ${address} error`, { error: event });
      subscriber.error(event);
    };

    ws.onclose = () => {
      log(`node ws ${address} closed`);
      subscriber.complete();
    };

    return () => {
      ws.close();
    };
  });
}

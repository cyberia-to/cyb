// eslint-disable-next-line max-classes-per-file
import { isDeliverTxSuccess } from '@cosmjs/stargate';
import { estimateOsmoFee } from '@osmonauts/utils';
import { cosmos, DeliverTxResponse } from 'osmojs';
import { useOsmosisSign } from '../context/OsmosisSignerProvider';

type Msg = {
  typeUrl: string;
  value: { [key: string]: any };
};

class TxError extends Error {
  constructor(message = 'Tx Error', options?: ErrorOptions) {
    super(message, options);
    this.name = 'TxError';
  }
}

class TxResult {
  error?: TxError;

  response?: DeliverTxResponse;

  constructor({ error, response }: Pick<TxResult, 'error' | 'response'>) {
    this.error = error;
    this.response = response;
  }

  get errorMsg() {
    return this.isOutOfGas
      ? `Out of gas. gasWanted: ${this.response?.gasWanted} gasUsed: ${this.response?.gasUsed}`
      : this.error?.message || 'Swap Failed';
  }

  get isSuccess() {
    return this.response && isDeliverTxSuccess(this.response);
  }

  get isOutOfGas() {
    return this.response && this.response.gasUsed > this.response.gasWanted;
  }
}

function useTx() {
  const { address, signingClient } = useOsmosisSign();

  async function tx(msgs: Msg[]) {
    if (!address || !signingClient) {
      return new TxResult({ error: new TxError('Wallet not connected') });
    }

    try {
      const txRaw = cosmos.tx.v1beta1.TxRaw;
      const fee = await estimateOsmoFee(signingClient, address, [...msgs], '');
      const signed = await signingClient.sign(address, [...msgs], fee, '');

      if (!signed) {
        return new TxResult({ error: new TxError('Invalid transaction') });
      }

      const response: any = await signingClient.broadcastTx(
        Uint8Array.from(txRaw.encode(signed).finish())
      );
      return isDeliverTxSuccess(response)
        ? new TxResult({ response })
        : new TxResult({ response, error: new TxError(response.rawLog) });
    } catch (e: any) {
      return new TxResult({ error: new TxError(e.message || 'Tx Error') });
    }
  }

  return { tx };
}

export default useTx;

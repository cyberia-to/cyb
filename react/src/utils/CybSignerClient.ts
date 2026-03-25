import { OfflineSigner } from '@cosmjs/proto-signing';
import { Tendermint34Client } from '@cosmjs/tendermint-rpc';
import {
  SigningCyberClient,
  SigningCyberClientOptions,
} from '@cybercongress/cyber-js';
import store from 'src/redux/store';
import { signerModalHandler } from 'src/services/signer/signer-modal-handler';

// eslint-disable-next-line import/prefer-default-export
export class CybSignerClient extends SigningCyberClient {
  signAndBroadcast(
    ...args: Parameters<SigningCyberClient['signAndBroadcast']>
  ) {
    return new Promise((resolve, reject) => {
      const [, messages, fee] = args;

      const { confirmation } = store.getState().signer;
      if (confirmation) {
        signerModalHandler.setSignRequestData('fee', fee as any);
        signerModalHandler.setSignRequestData('messages', [...messages]);
        signerModalHandler.setSignRequestData('resolve', resolve);
        signerModalHandler.setSignRequestData('reject', reject);

        signerModalHandler.openModal();
      } else {
        resolve({});
      }
    }).then(() => {
      const [signerAddress, messages, fee] = args;
      const { memo } = signerModalHandler.getData();

      return super
        .signAndBroadcast(signerAddress, messages, fee, memo ?? '')
        .finally(() => {
          signerModalHandler.closeModal();
          signerModalHandler.resetSignRequestData();
        });
    });
  }

  public static async connectWithSigner(
    endpoint: string,
    signer: OfflineSigner,
    options: SigningCyberClientOptions = {}
  ): Promise<SigningCyberClient> {
    const tmClient = await Tendermint34Client.connect(endpoint);
    return new CybSignerClient(tmClient as any, signer, options);
  }
}

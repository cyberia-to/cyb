/* eslint-disable import/no-unused-modules */
/* eslint-disable import/prefer-default-export */
import { EncodeObject, StdFee } from '@cybercongress/cyber-ts';
import type { SignerModalRef } from 'src/components/signer-modal/signer-modal';

interface SignerModalData {
  resolve?: (value: unknown) => void;
  reject?: (reason?: any) => void;
  signerAddress?: string;
  messages?: EncodeObject[];
  fee?: number | 'auto' | StdFee;
  memo?: string | undefined;
}

export class SignerModalHandler {
  private signerModalRef?: SignerModalRef;

  private data: SignerModalData = {};

  setSignerModalRef(signerModalRef: SignerModalRef) {
    this.signerModalRef = signerModalRef;
  }

  setSignRequestData<K extends keyof SignerModalData>(
    key: K,
    value: SignerModalData[K]
  ) {
    this.data = {
      ...this.data,
      [key]: value,
    };
  }

  resetSignRequestData() {
    this.data = {};
  }

  getData(): SignerModalData {
    return this.data;
  }

  openModal(): void {
    this.signerModalRef?.open();
  }

  closeModal(): void {
    this.signerModalRef?.close();
  }
}

export const signerModalHandler = new SignerModalHandler();

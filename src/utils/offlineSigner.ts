/* eslint-disable import/no-unused-modules */
/* eslint-disable import/prefer-default-export */
import {
  DirectSecp256k1HdWallet,
  DirectSecp256k1HdWalletOptions,
} from '@cosmjs/proto-signing/build/directsecp256k1hdwallet';
import { Bip39, EnglishMnemonic } from '@cosmjs/crypto';
import defaultNetworks from 'src/constants/defaultNetworks';
import networkList from './networkListIbc';

export class CybOfflineSigner extends DirectSecp256k1HdWallet {
  public static async fromMnemonic(
    mnemonic: string,
    options: Partial<DirectSecp256k1HdWalletOptions> = {}
  ): Promise<CybOfflineSigner> {
    const mnemonicChecked = new EnglishMnemonic(mnemonic);
    const seed = await Bip39.mnemonicToSeed(
      mnemonicChecked,
      options.bip39Password
    );

    const signer = new CybOfflineSigner(mnemonicChecked as any, {
      ...options,
      seed,
    });

    return signer;
  }
}

export const getOfflineSigner = (mnemonic: string, network?: string) =>
  CybOfflineSigner.fromMnemonic(mnemonic, {
    prefix: network
      ? networkList[network].prefix
      : defaultNetworks.bostrom.BECH32_PREFIX,
  });

export async function generateMnemonic(): Promise<string> {
  const wallet = await DirectSecp256k1HdWallet.generate(12, {
    prefix: defaultNetworks.bostrom.BECH32_PREFIX,
  });
  return wallet.mnemonic;
}

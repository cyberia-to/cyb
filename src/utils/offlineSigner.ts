import {
  DirectSecp256k1HdWallet,
  DirectSecp256k1HdWalletOptions,
} from '@cosmjs/proto-signing/build/directsecp256k1hdwallet';
import { DirectSecp256k1Wallet } from '@cosmjs/proto-signing';
import { Bip39, EnglishMnemonic } from '@cosmjs/crypto';
import { fromHex, toBase64, toUtf8 } from '@cosmjs/encoding';
import {
  Secp256k1HdWallet,
  Secp256k1Wallet,
  makeSignDoc,
} from '@cosmjs/amino';
import { AccountData, OfflineDirectSigner } from '@cosmjs/proto-signing';
import { SignDoc } from 'cosmjs-types/cosmos/tx/v1beta1/tx';
import { DirectSignResponse } from '@cosmjs/proto-signing';
import defaultNetworks from 'src/constants/defaultNetworks';
import networkList from './networkListIbc';

export interface ArbSignResult {
  pub_key: { type: string; value: string };
  signature: string;
}

export class CybOfflineSigner extends DirectSecp256k1HdWallet {
  private _aminoWallet: Secp256k1HdWallet | null = null;

  public static async fromMnemonic(
    mnemonic: string,
    options: Partial<DirectSecp256k1HdWalletOptions> = {}
  ): Promise<CybOfflineSigner> {
    const mnemonicChecked = new EnglishMnemonic(mnemonic);
    const seed = await Bip39.mnemonicToSeed(
      mnemonicChecked,
      options.bip39Password
    );

    // Parent constructor expects internal Mnemonic type — EnglishMnemonic satisfies it at runtime
    return new CybOfflineSigner(mnemonicChecked as unknown as string, {
      ...options,
      seed,
    });
  }

  private async getAminoWallet(): Promise<Secp256k1HdWallet> {
    if (!this._aminoWallet) {
      const [{ address }] = await this.getAccounts();
      const prefix = address.substring(0, address.indexOf('1'));
      this._aminoWallet = await Secp256k1HdWallet.fromMnemonic(
        this.mnemonic,
        { prefix }
      );
    }
    return this._aminoWallet;
  }

  /**
   * ADR-036 arbitrary message signing — mirrors Keplr's signArbitrary API.
   */
  async signArbitrary(
    _chainId: string,
    signerAddress: string,
    data: string
  ): Promise<ArbSignResult> {
    const aminoWallet = await this.getAminoWallet();

    // ADR-036 sign doc: wrap data as MsgSignData
    const signDoc = makeSignDoc(
      [
        {
          type: 'sign/MsgSignData',
          value: {
            signer: signerAddress,
            data: toBase64(toUtf8(data)),
          },
        },
      ],
      { amount: [], gas: '0' },
      '',
      '',
      0,
      0
    );

    const res = await aminoWallet.signAmino(signerAddress, signDoc);
    return res.signature;
  }
}

/** Type guard: checks if signer supports ADR-036 signArbitrary */
export function hasSignArbitrary(
  signer: unknown
): signer is CybOfflineSigner {
  return signer != null && typeof (signer as CybOfflineSigner).signArbitrary === 'function';
}

export async function generateMnemonic(): Promise<string> {
  const wallet = await DirectSecp256k1HdWallet.generate(24);
  return wallet.mnemonic;
}

export const getOfflineSigner = (mnemonic: string, network?: string) => {
  const prefix =
    network && networkList[network]?.prefix
      ? networkList[network].prefix
      : defaultNetworks.bostrom.BECH32_PREFIX;

  return CybOfflineSigner.fromMnemonic(mnemonic, { prefix });
};

export class CybPrivateKeySigner implements OfflineDirectSigner {
  private directWallet: DirectSecp256k1Wallet;
  private aminoWallet: Secp256k1Wallet;

  private constructor(
    directWallet: DirectSecp256k1Wallet,
    aminoWallet: Secp256k1Wallet
  ) {
    this.directWallet = directWallet;
    this.aminoWallet = aminoWallet;
  }

  static async fromKey(
    privkeyHex: string,
    prefix?: string
  ): Promise<CybPrivateKeySigner> {
    const privkey = fromHex(privkeyHex);
    const bech32Prefix = prefix || defaultNetworks.bostrom.BECH32_PREFIX;
    const [directWallet, aminoWallet] = await Promise.all([
      DirectSecp256k1Wallet.fromKey(privkey, bech32Prefix),
      Secp256k1Wallet.fromKey(privkey, bech32Prefix),
    ]);
    return new CybPrivateKeySigner(directWallet, aminoWallet);
  }

  async getAccounts(): Promise<readonly AccountData[]> {
    return this.directWallet.getAccounts();
  }

  async signDirect(
    address: string,
    signDoc: SignDoc
  ): Promise<DirectSignResponse> {
    return this.directWallet.signDirect(address, signDoc);
  }

  async signArbitrary(
    _chainId: string,
    signerAddress: string,
    data: string
  ): Promise<ArbSignResult> {
    const signDoc = makeSignDoc(
      [
        {
          type: 'sign/MsgSignData',
          value: {
            signer: signerAddress,
            data: toBase64(toUtf8(data)),
          },
        },
      ],
      { amount: [], gas: '0' },
      '',
      '',
      0,
      0
    );

    const res = await this.aminoWallet.signAmino(signerAddress, signDoc);
    return res.signature;
  }
}

export const getOfflineSignerFromPrivateKey = (
  privateKeyHex: string,
  network?: string
) => {
  const prefix =
    network && networkList[network]?.prefix
      ? networkList[network].prefix
      : defaultNetworks.bostrom.BECH32_PREFIX;

  return CybPrivateKeySigner.fromKey(privateKeyHex, prefix);
};

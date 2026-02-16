export type SubmitProofPayload = {
  hash: string;
  nonce: number;
  timestamp: number;
};

export type SubmitProofMsg = {
  submit_proof: SubmitProofPayload;
};

export type RelayProofRequest = SubmitProofPayload & {
  miner_address: string;
};

export type RelayProofResponse = {
  ok?: boolean;
  tx_hash?: string;
  error?: string;
};

export type SubmitErrorKind =
  | 'account_not_found'
  | 'transport'
  | 'contract'
  | 'unknown';

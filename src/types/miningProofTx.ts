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

export type LithiumEpochStatus = {
  epoch_id: number;
  start_height: number;
  end_height: number;
  proof_count: number;
  target_solutions: number;
  difficulty: number;
};

export type LithiumTargetResponse = {
  target_solutions: number;
};

export type LithiumProofStatsResponse = {
  epoch_id: number;
  proof_count: number;
  total_work: string;
};

export type LithiumMinerEpochStatsResponse = {
  address: string;
  epoch_id: number;
  proof_count: number;
};

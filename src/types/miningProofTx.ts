// === Legacy (SubmitProof) ===

export type SubmitProofPayload = {
  hash: string;
  nonce: number;
  timestamp: number;
};

export type SubmitProofMsg = {
  submit_proof: SubmitProofPayload;
};

// === Lithium v1 (SubmitLithiumProof) ===

export type SubmitLithiumProofMsg = {
  submit_lithium_proof: {
    hash: string;
    nonce: number;
    miner_address: string;
    block_hash: string;
    cyberlinks_merkle: string;
    epoch_id: number;
    timestamp: number;
    referrer?: string;
  };
};

// === Relay ===

export type RelayProofRequest = {
  hash: string;
  nonce: number;
  miner_address: string;
  block_hash: string;
  cyberlinks_merkle: string;
  epoch_id: number;
  timestamp: number;
  referrer?: string;
};

export type RelayProofResponse = {
  ok?: boolean;
  tx_hash?: string;
  error?: string;
};

// === Error classification ===

export type SubmitErrorKind =
  | 'account_not_found'
  | 'transport'
  | 'contract'
  | 'unknown';

// === Contract query response types ===

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

export type LithiumEmissionInfoResponse = {
  epoch_id: number;
  mining_emission: string;
  staking_emission: string;
  referral_emission: string;
  total_emission: string;
};

export type LithiumReferralInfoResponse = {
  address: string;
  referrer: string | null;
  referral_rewards: string;
  referrals_count: number;
};

export type LithiumStakeInfoResponse = {
  address: string;
  staked_amount: string;
  pending_unbonding: string;
  pending_unbonding_until: number;
  claimable_rewards: string;
};

export type BurnStatsResponse = {
  total_burned: string;
};

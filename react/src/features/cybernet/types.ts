export interface Metadata {
  name: string;
  particle: string;
  description: string;
  logo: string;
  types: string;
  extra: string;
}
export type SubnetInfo = {
  blocks_since_last_step: number;
  burn: number;
  difficulty: number;
  emission_values: number;
  immunity_period: number;
  kappa: number;
  max_allowed_uids: number;
  max_allowed_validators: number;
  max_weights_limit: number;
  metadata: string;
  min_allowed_weights: number;
  netuid: number;
  metadata: Metadata;
  network_modality: number;
  owner: string;
  rho: number;
  subnetwork_n: number;
  tempo: number;
};

export type Delegator = {
  delegate: string;
  take: number;
  nominators: [string, number][];
  owner: string;
  registrations: number[];
  validator_permits: number[];
  return_per_1000: number;
  total_daily_return: number;
  return_per_giga: {
    amount: number;
    denom: string;
  };
};

export type SubnetHyperParameters = {
  rho: number;
  kappa: number;
  immunity_period: number;
  min_allowed_weights: number;
  max_weights_limit: number;
  tempo: number;
  min_difficulty: number;
  max_difficulty: number;
  weights_version: number;
  weights_rate_limit: number;
  adjustment_interval: number;
  activity_cutoff: number;
  registration_allowed: boolean;
  target_regs_per_interval: number;
  min_burn: number;
  max_burn: number;
  bonds_moving_avg: number;
  max_regs_per_block: number;
};

export type StakeInfo = {
  coldkey: string;
  hotkey: string;
  stake: number;
}[];

type Stake = [string, number];

// [neuron_uid, weight]
export type Weight = [number, number];

interface AxonInfo {
  block: number;
  version: number;
  ip: string;
  port: number;
  ip_type: number;
  protocol: number;
  placeholder1: number;
  placeholder2: number;
}

interface PrometheusInfo {
  block: number;
  version: number;
  ip: string;
  port: number;
  ip_type: number;
}

export interface SubnetNeuron {
  hotkey: string;
  coldkey: string;
  uid: number;
  netuid: number;
  active: boolean;
  axon_info: AxonInfo;
  prometheus_info: PrometheusInfo;
  stake: Stake[];
  rank: number;
  emission: number;
  incentive: number;
  consensus: number;
  trust: number;
  validator_trust: number;
  dividends: number;
  last_update: number;
  validator_permit: boolean;
  weights: Weight[];
  bonds: any[];
  pruning_score: number;
}

// [neuron_uid, weight]
export type Weights = Weight[];

//genegated

interface BlockRewards {
  denom: string;
  amount: string;
}

interface TotalStake {
  denom: string;
  amount: string;
}

interface TotalIssuance {
  denom: string;
  amount: string;
}

interface TotalRewards {
  denom: string;
  amount: string;
}

export type ContractType = 'graph' | 'ml';

export enum ContractTypes {
  Graph = 'graph',
  ML = 'ml',
}

export interface Economy {
  validator_apr: string;
  staker_apr: string;
  block_rewards: BlockRewards;
  total_stake: TotalStake;
  default_commission: string;
  commission_change: boolean;
  total_issuance: TotalIssuance;
  total_rewards: TotalRewards;
}

export interface ContractWithData {
  address: string;
  type: ContractType;
  metadata: Metadata;
  economy: Economy;
  isLoading: boolean;
}

type StakeInfo2 = [string, number];

export type StakeList = StakeInfo2[];

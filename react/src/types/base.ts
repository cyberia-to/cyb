export type NeuronAddress = string;
export type ParticleCid = string;
export type TransactionHash = string;

export type CyberLinkSimple = {
  from: ParticleCid;
  to: ParticleCid;
};
export type Cyberlink = CyberLinkSimple & {
  timestamp: number;
  neuron: NeuronAddress;
};

export type CyberLinkNeuron = Omit<Cyberlink, 'timestamp'>;
export type CyberLinkTimestamp = Omit<Cyberlink, 'neuron'>;
export type CyberlinkTxHash = Cyberlink & { transaction_hash: string };

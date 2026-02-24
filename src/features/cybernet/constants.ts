const CYBERVER_CONTRACT_GRAPH = 'pussy1j9qku20ssfjdzgl3y5hl0vfxzsjwzwn7d7us2t2n4ejgc6pesqcqhnxsz0';
const CYBERVER_CONTRACT_ML = 'pussy1guj27rm0uj2mhwnnsr8j7cz6uvsz2d759kpalgqs60jahfzwgjcs4l28cw';

const CYBERVER_CONTRACT_GRAPH_LEGACY =
  'pussy155k695hqnzl05lx79kg9754k8cguw7wled38u2qacpxl62mrkfasy3k6x5';
const CYBERVER_CONTRACT_ML_LEGACY =
  'pussy1xemzpkq2qd6a5e08xxy5ffcwx9r4xn5fqe6v02rkte883f9xhg5q29ye9y';

const contracts = [CYBERVER_CONTRACT_GRAPH, CYBERVER_CONTRACT_ML];

if (process.env.NODE_ENV === 'development' || process.env.ENV === 'staging') {
  contracts.push(CYBERVER_CONTRACT_GRAPH_LEGACY, CYBERVER_CONTRACT_ML_LEGACY);
}

export const CYBERVER_CONTRACTS = contracts;

export const CYBERVER_CONTRACTS_LEGACY = [
  CYBERVER_CONTRACT_GRAPH_LEGACY,
  CYBERVER_CONTRACT_ML_LEGACY,
];

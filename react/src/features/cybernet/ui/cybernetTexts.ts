export const texts: {
  [key: string]: {
    default: string | { single: string; plural: string };
    graph: string | { single: string; plural: string };
  };
} = {
  contract: {
    default: 'contract',
    graph: 'verse',
  },
  root: {
    default: 'root',
    graph: 'board',
  },
  subnetwork: {
    default: 'subnet',
    graph: {
      single: 'faculty',
      plural: 'faculties',
    },
  },
  uid: {
    default: 'uid',
    graph: 'card',
  },
  contractOwner: {
    default: 'owner',
    graph: 'rector',
  },
  subnetOwner: {
    default: 'owner',
    graph: 'dean',
  },
  delegate: {
    default: 'delegate',
    graph: 'mentor',
  },
  delegator: {
    default: 'delegator',
    graph: 'learner',
  },
  validator: {
    default: 'validator',
    graph: 'professor',
  },
  rootValidator: {
    default: 'validator',
    graph: 'lead',
  },
  miner: {
    default: 'miner',
    graph: 'teacher',
  },
};

// fix
export type Texts =
  | 'contract'
  | 'root'
  | 'subnetwork'
  | 'uid'
  | 'contractOwner'
  | 'subnetOwner'
  | 'delegate'
  | 'delegator'
  | 'validator'
  | 'rootValidator'
  | 'miner';

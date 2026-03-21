import { LEDGER } from 'src/utils/config';

export const LIMIT = 15;

export const initialContentTypeFilterState = {
  text: false,
  image: false,
  video: false,
  pdf: false,
  link: false,
  // audio: false,
};

export enum ActionBarStates {
  STAGE_INIT = LEDGER.STAGE_INIT,
  STAGE_READY = LEDGER.STAGE_READY,
  STAGE_SUBMITTED = LEDGER.STAGE_SUBMITTED,
  STAGE_CONFIRMING = LEDGER.STAGE_CONFIRMING,
  STAGE_CONFIRMED = LEDGER.STAGE_CONFIRMED,
  STAGE_ERROR = LEDGER.STAGE_ERROR,

  STAGE_IPFS_HASH = 3.1,
  STAGE_KEPLR_APPROVE = 3.2,
}

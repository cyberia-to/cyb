import { ContractTypes } from '../../types';

export function checkIsMLVerse(type: ContractTypes) {
  return type === ContractTypes.ML;
}

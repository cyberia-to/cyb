/* eslint-disable */
import { createContext, useContext } from 'react';

const SigmaContext = createContext({
  totalCap: 0,
  changeCap: 0,
  updateTotalCap: () => {},
  updateChangeCap: () => {},
  updateDataCap: () => {},
  isOwner: false,
});

const useSigmaContext = () => {
  const context = useContext(SigmaContext);
  if (!context) {
    throw new Error('Error in creating the context');
  }
  return context;
};

export { useSigmaContext, SigmaContext };

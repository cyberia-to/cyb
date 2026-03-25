import { createContext, useContext } from 'react';
import { $TsFixMeFunc } from 'src/types/tsfix';

type SelectContextProps = {
  selectedOption: string;
  changeSelectedOption: $TsFixMeFunc;
};

const SelectContext = createContext<SelectContextProps>({
  selectedOption: '',
  // eslint-disable-next-line @typescript-eslint/no-empty-function
  changeSelectedOption: () => {},
});

const useSelectContext = () => {
  const context = useContext(SelectContext);
  if (!context) {
    throw new Error('Error in creating the context');
  }
  return context;
};

export { useSelectContext, SelectContext };

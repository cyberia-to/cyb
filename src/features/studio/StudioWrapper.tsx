import { MilkdownProvider } from '@milkdown/react';
import Studio from './Studio';
import StudioContextProvider from './studio.context';

function StudioWrapper() {
  return (
    <StudioContextProvider>
      <MilkdownProvider>
        <Studio />
      </MilkdownProvider>
    </StudioContextProvider>
  );
}

export default StudioWrapper;

import React, { useEffect, useMemo, useState } from 'react';
import { isMobileTablet } from 'src/utils/utils';

type DeviceContextType = {
  isMobile: boolean;
  viewportWidth: number;
};

const DeviceContext = React.createContext<DeviceContextType>({
  isMobile: isMobileTablet(),
  viewportWidth: window.innerWidth,
});

export function useDevice() {
  return React.useContext(DeviceContext);
}

function DeviceProvider({ children }: { children: React.ReactNode }) {
  const [viewportWidth, setViewportWidth] = useState(window.innerWidth);

  const isMobile = isMobileTablet() && viewportWidth < 768;

  useEffect(() => {
    function handleResize() {
      setViewportWidth(window.innerWidth);
    }

    window.addEventListener('resize', handleResize);
    return () => {
      window.removeEventListener('resize', handleResize);
    };
  }, []);

  const value = useMemo(() => {
    return {
      isMobile,
      viewportWidth,
    };
  }, [isMobile, viewportWidth]);

  return (
    <DeviceContext.Provider value={value}>{children}</DeviceContext.Provider>
  );
}

export default DeviceProvider;

import React, { createContext, useEffect, useMemo, useState } from 'react';
import { useLocation } from 'react-router-dom';

/*
  copied from, in future maybe will be possible to delete this file
  https://github.com/remix-run/react-router/discussions/9860
*/

interface IPreviousPageContextData {
  previousPathname: string | undefined;
  /** Is true if the previous page has the same origin as the current page */
  previousPageIsAppPage: boolean;
  /** Is true when the previous page is lower in the path-tree */
  previousPageIsShallower: boolean;
}

const PreviousPageContext = createContext<IPreviousPageContextData | undefined>(undefined);

export function usePreviousPage() {
  //   if (context === undefined) {
  //     throw new Error(
  //       'usePreviousPage must be used within a PreviousPageProvider'
  //     );
  //   }
  return React.useContext(PreviousPageContext);
  //   return context;
}

interface IPreviousPageProps {
  children: React.ReactNode;
}

function PreviousPageProvider({ children }: IPreviousPageProps) {
  const [currentPathname, setCurrentPathname] = useState<string | undefined>(undefined);
  const [previousPathname, setPreviousPathname] = useState<string | undefined>(undefined);
  const { pathname: pathnameState } = useLocation();

  useEffect(() => {
    // Guard
    if (currentPathname !== window.location.pathname) {
      // Update pathname
      if (currentPathname !== undefined) {
        setPreviousPathname(currentPathname);
      }

      setCurrentPathname(window.location.pathname + window.location.search);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentPathname]);

  const providerValue = useMemo(
    () => ({
      previousPageIsAppPage: previousPathname !== undefined,
      previousPageIsShallower:
        previousPathname !== undefined && previousPathname.length < window.location.pathname.length,
      previousPathname,
    }),
    [previousPathname]
  );

  return (
    // This component will be used to encapsulate the whole App,
    // so all components will have access to the Context
    <PreviousPageContext.Provider value={providerValue}>{children}</PreviousPageContext.Provider>
  );
}
export { PreviousPageContext, PreviousPageProvider };

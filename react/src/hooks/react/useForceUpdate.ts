import React, { useCallback } from 'react';

function useForceUpdate() {
  const [, setTick] = React.useState(0);

  const update = useCallback(() => {
    setTick((tick) => tick + 1);
  }, []);

  return {
    forceUpdate: update,
  };
}

export default useForceUpdate;

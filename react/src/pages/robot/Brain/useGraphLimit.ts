import { useCallback, useEffect } from 'react';
import { useSearchParams } from 'react-router-dom';

const DEFAULT_LIMIT = 10000;

// WIP
function useGraphLimit(initialLimit?: number) {
  const [searchParams, setSearchParams] = useSearchParams();

  const limit = Number(searchParams.get('limit')) || initialLimit || DEFAULT_LIMIT;

  const style = searchParams.get('style');

  const addLimit = useCallback(
    (limit: number) => {
      setSearchParams((prev) => {
        const next = new URLSearchParams(prev);
        next.set('limit', String(limit));
        return next;
      });
    },
    [setSearchParams]
  );

  useEffect(() => {
    addLimit(limit);
  }, [limit, addLimit]);

  return {
    limit,
    isCurvedStyle: style === 'curved',
    setSearchParams,
    setLimit: (limit: number) => {
      addLimit(limit);
    },
  };
}

export default useGraphLimit;

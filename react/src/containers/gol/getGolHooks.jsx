import { useEffect, useState } from 'react';
import finalResultGoL from '../../utils/finalResultGoL';

function useGetGol(address) {
  const [resultGol, setResultGol] = useState({});
  const [totalGol, setTotalGol] = useState(0);

  useEffect(() => {
    if (address && address !== 0) {
      if (Object.hasOwn(finalResultGoL, address)) {
        const resultGolData = finalResultGoL[address];
        setResultGol((item) => ({ ...item, ...resultGolData }));
        if (resultGolData.sum) {
          setTotalGol(resultGolData.sum);
        }
      } else {
        setTotalGol(0);
        setResultGol({});
      }
    }
  }, [address]);

  return { totalGol, resultGol };
}

export default useGetGol;

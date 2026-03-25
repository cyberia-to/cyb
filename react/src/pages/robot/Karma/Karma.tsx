import { useEffect } from 'react';
import { useAdviser } from 'src/features/adviser/context';
import UnderConstruction from '../UnderConstruction/UnderConstruction';

function Karma() {
  const { setAdviser } = useAdviser();

  useEffect(() => {
    setAdviser(
      <>
        the invisible power of cyber graph influence <br />
        more karma more particles weight
      </>
    );
  }, [setAdviser]);

  return <UnderConstruction />;
}

export default Karma;

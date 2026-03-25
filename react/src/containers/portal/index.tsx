import { useEffect } from 'react';
import { useAdviser } from 'src/features/adviser/context';
import usePassportByAddress from 'src/features/passport/hooks/usePassportByAddress';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppSelector } from 'src/redux/hooks';
import GetCitizenship from './citizenship';
import PassportMoonCitizenship from './PasportMoonCitizenship';

function PortalCitizenship() {
  const addressActive = useAppSelector(selectCurrentAddress);
  const { loading, passport } = usePassportByAddress(addressActive);

  const { setAdviser } = useAdviser();

  useEffect(() => {
    if (loading) {
      setAdviser('Loading...', 'yellow');
    } else {
      setAdviser('');
    }
  }, [setAdviser, loading]);

  if (loading) {
    return null;
  }

  if (!passport) {
    return <GetCitizenship />;
  }
  return <PassportMoonCitizenship />;
}

export default PortalCitizenship;

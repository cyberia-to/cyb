import { useNavigate, useParams } from 'react-router-dom';
import { PATTERN_CYBER } from 'src/constants/patterns';
import usePassportByAddress from 'src/features/passport/hooks/usePassportByAddress';
import usePassportContract from 'src/features/passport/usePassportContract';
import { useRobotContext } from 'src/pages/robot/robot.context';
import { Citizenship } from 'src/types/citizenship';
import Sense from './Sense';

/**
    Complex logic, although code seems simple
    change carefully

    Handles such routes:
        - :nickname/sense/:address
        - :nickname/sense/:nickname
        - :nickname/sense/:cid
*/

const addressToNicknameMap: {
  [key: string]: string;
} = {};

function findAddressByNickname(nickname: string) {
  return Object.entries(addressToNicknameMap).find(([, value]) => value === nickname)?.[0];
}

function handlePassport(passport: Citizenship | null) {
  if (!passport) {
    return;
  }

  const {
    owner,
    extension: { nickname },
  } = passport;

  addressToNicknameMap[owner] = nickname;
}

function SenseRoutingWrapper() {
  const { senseId: paramSenseId } = useParams<{
    senseId: string;
  }>();
  const { isOwner, address: robotAddress } = useRobotContext();

  const address = paramSenseId?.match(PATTERN_CYBER) ? paramSenseId : undefined;

  const nickname = paramSenseId?.includes('@') ? paramSenseId.replace('@', '') : undefined;

  const { data: passportByNickname } = usePassportContract<Citizenship | null>({
    query: {
      passport_by_nickname: {
        nickname: nickname!,
      },
    },
    skip: !nickname || Boolean(findAddressByNickname(nickname)),
  });

  const { data: passportByAddress } = usePassportByAddress(address, {
    skip: Boolean(address && addressToNicknameMap[address]),
  });

  if (passportByNickname) {
    handlePassport(passportByNickname);
  }
  if (passportByAddress) {
    handlePassport(passportByAddress);
  }

  const navigate = useNavigate();

  if (address && addressToNicknameMap[address]) {
    navigate(`../@${addressToNicknameMap[address]}`, {
      relative: 'path',
      replace: true,
    });
  }

  let senseId = address || (nickname && findAddressByNickname(nickname)) || paramSenseId;

  if (!isOwner) {
    senseId = robotAddress || undefined;
  }

  return <Sense urlSenseId={senseId} />;
}

export default SenseRoutingWrapper;

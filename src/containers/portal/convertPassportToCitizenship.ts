import { Citizenship } from 'src/types/citizenship';

const convertPassportToCitizenship = (
  passport: any,
  owner: string
): Citizenship => {
  return {
    owner: owner,
    extension: {
      nickname: passport.extension?.nickname,
      avatar: passport.extension?.avatar,
      addresses: passport.extension?.addresses,
    },
  };
};

export default convertPassportToCitizenship;

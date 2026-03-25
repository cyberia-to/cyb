// https://docs.rs/cw721-base/latest/cw721_base/state/struct.TokenInfo.html
export type Citizenship = {
  owner: string;
  // fix approvals type when will need
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  approvals: any[];
  token_uri: string | null;
  // https://github.com/cybercongress/cw-cybergift/blob/main/contracts/cw-cyber-passport/schema/passport_metadata.json
  extension: {
    avatar: string;
    nickname: string;
    addresses:
      | {
          address: string;
          label: string | null;
        }[]
      | null;
    data: string | null;
    particle: string | null;
  };
};

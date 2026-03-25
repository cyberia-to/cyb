export type ClaimMsg = {
  claim: {
    proof: string[];
    gift_amount: string;
    gift_claiming_address: string;
    nickname: string;
  };
};

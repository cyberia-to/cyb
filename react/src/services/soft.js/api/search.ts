export type SearchItemResult = {
  particle: string;
  rank: number;
  grade: {
    from: number;
    to: number;
    value: number;
  };
  status: 'impossibleLoad';
  query: string;
  text: string;
  content: boolean;
};

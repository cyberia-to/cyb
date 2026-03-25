export enum TypeRecipient {
  friends = 'friends',
  my = 'my',
  following = 'following',
}

export type DataItem = {
  nickname: string | undefined;
  cidAvatar: undefined | string;
  owner: string;
};

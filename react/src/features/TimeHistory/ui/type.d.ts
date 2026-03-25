export type RouteItemT = {
  url: string;
};

export type Tx = {
  type: string;
  value: any;
};

export type ActionType = 'route' | 'tx';

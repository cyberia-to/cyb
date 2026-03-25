export enum LinksTypeFilter {
  to = 'to',
  from = 'from',
  all = 'all',
}

export type LinksType = Exclude<LinksTypeFilter, LinksTypeFilter.all>;

export type SearchItem = {
  cid: string;
  rank?: string;
  grade?: string;
  timestamp?: string;
  type?: LinksTypeFilter;
};

export enum SortBy {
  rank = 'rank',
  date = 'date',
  // not ready
  // popular = 'popular',
  // mine = 'mine',
}

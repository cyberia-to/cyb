import { SearchItem } from '../types';

// eslint-disable-next-line import/prefer-default-export
export function merge(fromArr: SearchItem[], toArr: SearchItem[]) {
  // maybe not good algorithm
  const fromCids = fromArr.map((item) => item.cid);
  const toCids = toArr.map((item) => item.cid);

  const allCids = fromCids.filter((item) => toCids.includes(item));

  const added: string[] = [];

  return fromArr.concat(toArr).reduce<SearchItem[]>((acc, item) => {
    if (added.includes(item.cid)) {
      return acc;
    }

    if (allCids.includes(item.cid)) {
      added.push(item.cid);

      return acc.concat({
        ...item,
        type: 'all',
      });
    }

    return acc.concat(item);
  }, []);
}

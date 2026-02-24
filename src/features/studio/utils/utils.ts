import { getTransactions } from 'src/services/transactions/lcd';
import { CyberLinkSimple } from 'src/types/base';
import { KeywordsItem } from '../studio.context';

export const checkLoopLinks = async (links: CyberLinkSimple[]) => {
  const uniqueLinks: CyberLinkSimple[] = [];
  const loopLink: CyberLinkSimple[] = [];

  for (let index = 0; index < links.length; index++) {
    const item = links[index];
    // eslint-disable-next-line no-await-in-loop
    const response = await getTransactions({
      events: [
        { key: 'cyberlink.particleFrom', value: item.from },
        { key: 'cyberlink.particleTo', value: item.to },
      ],
      pagination: { limit: 1, offset: 0 },
    });

    if (!response?.txs.length) {
      uniqueLinks.push(item);
    } else {
      loopLink.push(item);
    }
  }

  return { uniqueLinks, loopLink };
};

export const mapLinks = (
  cid: string,
  keywords: {
    from: KeywordsItem[];
    to: KeywordsItem[];
  }
) => {
  return [
    ...keywords.from.map((item) => ({
      from: item.cid,
      to: cid,
    })),
    ...keywords.to.map((item) => ({
      from: cid,
      to: item.cid,
    })),
  ];
};

export const reduceLoopKeywords = (loopLink: CyberLinkSimple[], keywords: KeywordsItem[]) => {
  const keywordsArr: string[] = [];

  loopLink.forEach((itemLink) => {
    keywords.forEach((itemKey) => {
      if (itemKey.cid === itemLink.from || itemKey.cid === itemLink.to) {
        keywordsArr.push(itemKey.text);
      }
    });
  });

  return keywordsArr;
};

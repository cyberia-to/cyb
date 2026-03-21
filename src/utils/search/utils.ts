import { CyberClient } from '@cybercongress/cyber-js';
import axios from 'axios';
import { LCD_URL } from 'src/constants/config';
import { PATTERN_IPFS_HASH } from 'src/constants/patterns';
import { ParticleCid } from 'src/types/base';

import { getIpfsHash } from '../ipfs/helpers';
import { encodeSlash } from '../utils';

// TODO: move rank to features/rank
export const formatNumber = (number, toFixed) => {
  let formatted = +number;

  if (toFixed) {
    formatted = +formatted.toFixed(toFixed);
  }

  return formatted.toLocaleString('en').replace(/,/g, ' ');
};

export const getRankGrade = (rank) => {
  let from;
  let to;
  let value;

  if (rank > 0.00000276) {
    from = 0.00000276;
    to = 0.01;
    value = 1;
  } else if (rank > 0.00000254879356777504 && rank <= 0.00000276) {
    from = 0.00000254879356777504;
    to = 0.00000276;
    value = 2;
  } else if (rank > 0.00000233758713555007 && rank <= 0.00000254879356777504) {
    from = 0.00000233758713555007;
    to = 0.00000254879356777504;
    value = 3;
  } else if (rank > 0.00000191517427110014 && rank <= 0.00000233758713555007) {
    from = 0.00000191517427110014;
    to = 0.00000233758713555007;
    value = 4;
  } else if (rank > 0.00000128155497442525 && rank <= 0.00000191517427110014) {
    from = 0.00000128155497442525;
    to = 0.00000191517427110014;
    value = 5;
  } else if (rank > 0.00000022552281330043 && rank <= 0.00000128155497442525) {
    from = 0.00000022552281330043;
    to = 0.00000128155497442525;
    value = 6;
  } else if (rank > 0 && rank <= 0.00000022552281330043) {
    from = 0;
    to = 0.00000022552281330043;
    value = 7;
  } else {
    from = 'n/a';
    to = 'n/a';
    value = 'n/a';
  }

  return {
    from,
    to,
    value,
  };
};

export const getRelevance = async (page = 0, limit = 50) => {
  try {
    const response = await axios({
      method: 'get',
      url: `${LCD_URL}/cyber/rank/v1beta1/rank/top`,
      params: {
        'pagination.page': page,
        'pagination.perPage': limit,
      },
    });
    return response.data;
  } catch (_error) {
    return {};
  }
};

export const keybaseCheck = async (identity) => {
  try {
    const response = await axios({
      method: 'get',
      url: `https://keybase.io/_/api/1.0/user/lookup.json?key_suffix=${identity}&fields=basics`,
    });
    return response.data;
  } catch (e) {
    console.log(e);
    return null;
  }
};

export const keybaseAvatar = async (identity: string) => {
  try {
    const response = await axios({
      method: 'get',
      url: `https://keybase.io/_/api/1.0/user/lookup.json?key_suffix=${identity}&fields=pictures`,
    });
    return response.data;
  } catch (_e) {
    return undefined;
  }
};

export const authAccounts = async (address) => {
  try {
    const response = await axios({
      method: 'get',
      url: `${LCD_URL}/cosmos/auth/v1beta1/accounts/${address}`,
    });
    return response.data;
  } catch (error) {
    console.log(error);
    return null;
  }
};

// Access-Control-Allow-Origin
export const getCredit = async (address) => {
  try {
    const headers = {
      'Content-Type': 'application/json',
    };
    const fromData = {
      denom: 'boot',
      address,
    };
    const response = await axios({
      method: 'post',
      // url: 'http://localhost:8000/credit',
      url: 'https://titan.cybernode.ai/credit',
      headers,
      data: JSON.stringify(fromData),
    });

    return response;
  } catch (_error) {
    return null;
  }
};

export const getSearchQuery = async (query: ParticleCid | string) =>
  query.match(PATTERN_IPFS_HASH) ? query : getIpfsHash(encodeSlash(query));

export const searchByHash = async (client: CyberClient, hash: string, page: number) => {
  try {
    const results = await client.search(hash, page);

    return results;
  } catch (error) {
    // TODO: handle
    console.error(error);
    return undefined;
  }
};

// don't add funcs more

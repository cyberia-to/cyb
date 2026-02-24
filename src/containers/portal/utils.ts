import { CyberClient } from '@cybercongress/cyber-js';
import { useQuery } from '@tanstack/react-query';
import axios from 'axios';
import { useEffect } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';
import { getPassport } from 'src/services/passports/lcd';
import { Nullable } from 'src/types';
import { Citizenship } from 'src/types/citizenship';
import { AccountValue } from 'src/types/defaultAccount';

const AMOUNT_ALL_STAGE = 90;
const NEW_RELEASE = 1000; // release 1% every 1k claims
const CONSTITUTION_HASH = 'QmcHB9GKHAKCLQhmSj71qNJhENJJg8Gymd1PvvsCQBhG7M';

// test root
// const CONTRACT_ADDRESS_GIFT =
//   'bostrom1dwzfa74hzpt6393czajlldnxjup8zk3xh3skegnm67yzqx33k2cssyduk8';
// const CONTRACT_ADDRESS_PASSPORT =
//   'bostrom1fzm6gzyccl8jvdv3qq6hp9vs6ylaruervs4m06c7k0ntzn2f8faq7ha2z2';

// prod root
const CONTRACT_ADDRESS_GIFT = 'bostrom16t6tucgcqdmegye6c9ltlkr237z8yfndmasrhvh7ucrfuqaev6xq7cpvek';
const CONTRACT_ADDRESS_PASSPORT =
  'bostrom1xut80d09q0tgtch8p0z4k5f88d3uvt8cvtzm5h3tu3tsy4jk9xlsfzhxel';

const DICTIONARY = {
  Astronauts: 'Astronaut',
  'Average Citizens. ETH Analysis': 'Average Citizens',
  'Cyberpunks. ERC20 and ERC721 Analysis': 'Cyberpunk',
  'Extraordinary Hackers. Gas Analysis': 'Extraordinary Hacker',
  'Key Opinion Leaders. ERC20 Analysis': 'Key Opinion Leader',
  'Masters of the Great Web. Gas and ERC721 Analysis': 'Master of the Great Web',
  'Passionate Investors. ERC20 Analysis': 'Passionate Investor',
  'Heroes of the Great Web. Genesis and ETH2 Stakers': 'True Hero of the Great Web',
  Leeches: 'Devil',
};

const GIFT_ICON = '🎁';
const BOOT_ICON = '🟢';

const useGetActivePassport = (addressActive: Nullable<AccountValue>, updateFunc?: number) => {
  const queryClient = useQueryClient();
  const data = useQuery(
    ['active_passport', addressActive?.bech32],
    () => activePassport(queryClient, addressActive?.bech32),
    {
      enabled: Boolean(addressActive) && Boolean(queryClient),
    }
  );

  useEffect(() => {
    if (updateFunc) {
      data.refetch();
    }
  }, [updateFunc, data.refetch]);

  return {
    citizenship: data.data,
    loading: data.isFetching,
  };
};

// TODO: replace with hook
const activePassport = async (
  client: CyberClient,
  address: string
): Promise<Nullable<Citizenship>> => {
  try {
    const query = {
      active_passport: {
        address,
      },
    };
    const response = await client.queryContractSmart(CONTRACT_ADDRESS_PASSPORT, query);
    return response;
  } catch (error) {
    console.log('error', error);
    return null;
  }
};

const parseValue = (data) => {
  if (data.length > 0) {
    const newData = data.replace(/'/g, '"');
    return JSON.parse(newData);
  }
  return null;
};

const parseValueDetails = (data) => {
  const value = parseValue(data);
  if (value !== null) {
    const details = {};
    value.forEach((item) => {
      details[DICTIONARY[item.audience]] = { gift: item.gift };
    });
    return details;
  }
  return null;
};

const parseResponse = (obj) => {
  return {
    ...obj,
    details: parseValueDetails(obj.details),
    proof: parseValue(obj.proof),
  };
};

const checkGift = async (address) => {
  try {
    const response = await axios({
      method: 'GET',
      url: `https://titan.cybernode.ai/graphql/api/rest/get-cybergift/${address}`, // prod root
      // url: `https://titan.cybernode.ai/graphql/api/rest/get-test-gift/${address}`, // test root
    });

    if (response?.data) {
      const { data } = response;
      if (
        Object.hasOwn(data, 'cyber_gift_proofs') &&
        Object.keys(data.cyber_gift_proofs).length > 0
      ) {
        const { cyber_gift_proofs: cyberGiftData } = data;
        return parseResponse(cyberGiftData[0]);
      }
      if (Object.hasOwn(data, 'test_gift') && Object.keys(data.test_gift).length > 0) {
        const { test_gift: cyberGiftData } = data;
        return parseResponse(cyberGiftData[0]);
      }
    }
    return null;
  } catch (_error) {
    return null;
  }
};

const queryContractSmartPassport = async (_client, query) => {
  try {
    const response = await getPassport(query);
    // const response = await client.queryContractSmart(
    //   CONTRACT_ADDRESS_PASSPORT,
    //   query
    // );
    return response;
  } catch (error) {
    console.log('error', error);
    return null;
  }
};

const queryContractSmartGift = async (client, query) => {
  try {
    const response = await client.queryContractSmart(CONTRACT_ADDRESS_GIFT, query);
    return response;
  } catch (error) {
    console.log('error', error);
    return null;
  }
};

const getStateGift = async (client) => {
  try {
    const query = {
      state: {},
    };
    const response = await queryContractSmartGift(client, query);
    return response;
  } catch (error) {
    console.log('error', error);
    return null;
  }
};

const getConfigGift = async (client) => {
  try {
    const query = {
      config: {},
    };
    const response = await queryContractSmartGift(client, query);
    return response;
  } catch (error) {
    console.log('error', error);
    return null;
  }
};

const getReleaseState = async (client, address) => {
  try {
    const query = {
      release_state: {
        address,
      },
    };
    const response = await queryContractSmartGift(client, query);
    return response;
  } catch (error) {
    console.log('error', error);
    return null;
  }
};

const getClaimedAmount = async (client, address) => {
  try {
    const query = {
      claim: {
        address,
      },
    };
    const response = await queryContractSmartGift(client, query);
    return response;
  } catch (error) {
    console.log('error', error);
    return null;
  }
};

const getIsClaimed = async (client, address) => {
  try {
    const query = {
      is_claimed: {
        address,
      },
    };
    const response = await queryContractSmartGift(client, query);
    return response;
  } catch (error) {
    console.log('error', error);
    return null;
  }
};

const getPassportByNickname = async (client, nickname) => {
  try {
    const query = {
      passport_by_nickname: {
        nickname,
      },
    };
    const response = await queryContractSmartPassport(client, query);
    return response;
  } catch (error) {
    console.log('error', error);
    return null;
  }
};

const getNumTokens = async (client) => {
  try {
    const query = {
      num_tokens: {},
    };
    const response = await queryContractSmartPassport(client, query);
    return response;
  } catch (error) {
    console.log('error', error);
    return null;
  }
};

const tooMuthAddressError =
  'failed to execute message; message index: 0: Address is not eligible to claim airdrop, Too many addresses: execute wasm contract failed';

const canProve8AddressNewError = 'You can prove only 8 addresses for one passport';

const parseRowLog = (rawlog) => {
  if (rawlog === tooMuthAddressError) {
    return canProve8AddressNewError;
  }

  return rawlog;
};

export {
  activePassport,
  CONTRACT_ADDRESS_PASSPORT,
  useGetActivePassport,
  CONSTITUTION_HASH,
  CONTRACT_ADDRESS_GIFT,
  GIFT_ICON,
  BOOT_ICON,
  AMOUNT_ALL_STAGE,
  NEW_RELEASE,
  checkGift,
  getConfigGift,
  getStateGift,
  getReleaseState,
  getClaimedAmount,
  getIsClaimed,
  getPassportByNickname,
  parseRowLog,
  getNumTokens,
};

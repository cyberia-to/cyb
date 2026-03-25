import { useCallback, useEffect, useState } from 'react';
import { CYBER_CONGRESS_ADDRESS } from 'src/constants/app';
import { BECH32_PREFIX } from 'src/constants/config';

import { PATTERN_CYBER } from 'src/constants/patterns';
import useQueueIpfsContent from 'src/hooks/useQueueIpfsContent';
import { getFollows, getTweet } from 'src/services/transactions/lcd';
import { Option } from 'src/types';
import { fromBech32 } from '../../utils/utils';

interface Tweet {
  status: string;
  text: string;
  address: string;
  content: boolean;
  time: string;
  rank: null | number;
}

type TweetTemp = {
  [key: string]: Tweet;
};

const useGetTweets = (addressActive: Option<string>) => {
  const [tweets, setTweets] = useState<TweetTemp>({});
  const [tweetData, setTweetData] = useState([]);
  const [loadingTweets, setLoadingTweets] = useState(false);
  const { fetchParticleAsync } = useQueueIpfsContent(addressActive);

  const getFollow = useCallback(
    async (responseFollows) => {
      responseFollows.txs.forEach(async (item) => {
        const cid = item.tx.value.msg[0].value.links[0].to;

        const addressResolve = fetchParticleAsync
          ? (await fetchParticleAsync(cid))?.result?.textPreview
          : undefined;

        if (addressResolve?.match(PATTERN_CYBER)) {
          const responseTweet = await getTweet(addressResolve);
          if (responseTweet?.txs && responseTweet.txs.length > 0) {
            setTweetData((items) => [...items, ...responseTweet.txs]);
          }
        }
      });

      setLoadingTweets(false);
    },
    [fetchParticleAsync]
  );

  useEffect(() => {
    (async () => {
      setLoadingTweets(true);
      let responseFollows = null;
      if (addressActive?.match(PATTERN_CYBER)) {
        responseFollows = await getFollows(addressActive);
        if (responseFollows !== null && responseFollows.total_count > 0) {
          getFollow(responseFollows);
        }
      }
      if (!addressActive || !responseFollows || Number(responseFollows.total_count) === 0) {
        const cyberCongressAddress = fromBech32(CYBER_CONGRESS_ADDRESS, BECH32_PREFIX);

        const responseTwit = await getTweet(cyberCongressAddress);
        if (responseTwit && responseTwit.total_count > 0) {
          setTweetData((item) => [...item, ...responseTwit.txs]);
        }
      }
      setLoadingTweets(false);
    })();
  }, [getFollow, addressActive]);

  useEffect(() => {
    let tweetTemp = {};
    if (Object.keys(tweetData).length > 0) {
      tweetTemp = tweetData.reduce(
        (obj, item) => ({
          ...obj,
          [item.tx.value.msg[0].value.links[0].to]: {
            status: 'impossibleLoad',
            text: item.tx.value.msg[0].value.links[0].to,
            address: item.tx.value.msg[0].value.address,
            content: false,
            time: item.timestamp,
            rank: null,
          },
        }),
        {}
      );
      setTweets(tweetTemp as TweetTemp);
      setLoadingTweets(false);
    }
  }, [tweetData]);

  return {
    tweets,
    loadingTweets,
  };
};

export default useGetTweets;

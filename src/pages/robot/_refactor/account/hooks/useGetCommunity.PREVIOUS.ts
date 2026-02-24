import { useEffect, useState } from 'react';
import { PATTERN_CYBER } from 'src/constants/patterns';
import { useBackend } from 'src/contexts/backend/backend';
import useQueueIpfsContent from 'src/hooks/useQueueIpfsContent';
import { getIpfsHash } from 'src/utils/ipfs/helpers';

function useGetCommunity(address: string | null, skip?: boolean) {
  const { fetchParticleAsync } = useQueueIpfsContent();
  const { dbApi } = useBackend();
  const [followers, setFollowers] = useState([]);
  const [following, setFollowing] = useState([]);
  const [community, setCommunity] = useState({
    following: [],
    followers: [],
    friends: [],
  });

  // TODO: maybe refactor
  const [loading, setLoading] = useState({
    following: false,
    followers: false,
    friends: false,
  });

  useEffect(() => {
    const getFollowersFunc = async () => {
      let responseFollows = null;
      setFollowers([]);
      setLoading((state) => ({ ...state, followers: true }));
      if (address) {
        const addressHash = await getIpfsHash(address);
        responseFollows = await getFollowers(addressHash);
      }

      if (responseFollows?.txs) {
        responseFollows.txs.forEach(async (item) => {
          const addressFollowers = item.tx.value.msg[0].value.neuron;
          if (!addressFollowers) {
            // maybe better filter by cyber/MsgCyberlink type?
            return;
          }
          setFollowers((items) => [...items, addressFollowers]);
        });
      }
      setLoading((state) => ({ ...state, followers: false }));
    };

    if (!address || skip) {
      return;
    }

    getFollowersFunc();
  }, [address, skip]);

  useEffect(() => {
    const getFollowersAddress = async () => {
      let responseFollows = null;
      setFollowing([]);
      setLoading((state) => ({ ...state, following: true }));
      if (address) {
        responseFollows = await getFollows(address);
      }

      if (responseFollows?.txs) {
        responseFollows.txs.forEach(async (item) => {
          const cid = item.tx.value.msg[0].value.links[0].to;

          // TODO: ipfs refactor
          const addressResolve = fetchParticleAsync
            ? (await fetchParticleAsync(cid))?.result?.textPreview
            : undefined;

          if (addressResolve) {
            const addressFollow = addressResolve;
            // console.log('addressResolve :>> ', addressResolve);
            if (addressFollow.match(PATTERN_CYBER)) {
              setFollowing((items) => [...items, addressFollow]);
            }
          }
        });
      }
      setLoading((state) => ({ ...state, following: false }));
    };
    if (!address || skip) {
      return;
    }

    getFollowersAddress();
  }, [address, skip, fetchParticleAsync]);

  useEffect(() => {
    setCommunity({
      following: [],
      followers: [],
      friends: [],
    });
    setLoading((state) => ({ ...state, friends: true }));

    if (following.length > 0 && followers.length > 0) {
      const followingArr = [];
      const followersArr = followers.slice();
      const friendsArr = [];
      following.forEach((item) => {
        if (followersArr.indexOf(item) !== -1) {
          const index = followersArr.indexOf(item);
          followersArr.splice(index, 1);
          friendsArr.push(item);
        } else {
          followingArr.push(item);
        }
      });
      setCommunity({
        following: followingArr,
        followers: followersArr,
        friends: friendsArr,
      });
    } else {
      if (following.length > 0 && followers.length === 0) {
        setCommunity((itemState) => ({
          ...itemState,
          following,
        }));
      }
      if (following.length === 0 && followers.length > 0) {
        setCommunity((itemState) => ({
          ...itemState,
          followers,
        }));
      }
    }
    setLoading((state) => ({ ...state, friends: false }));
  }, [following, followers]);

  return {
    community,
    loading: {
      ...loading,
      friends: loading.friends || loading.followers || loading.following,
    },
  };
}

export default useGetCommunity;

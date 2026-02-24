import { useEffect, useState } from 'react';
import { useSelector } from 'react-redux';
import { useLocation } from 'react-router-dom';
import { useGetKarma } from 'src/containers/application/Karma/useGetKarma';
import useGetGol from 'src/containers/gol/getGolHooks';
import { useGetBalanceBostrom } from 'src/containers/sigma/hooks';
import { useQueryClient } from 'src/contexts/queryClient';
import { useGetIpfsInfo } from 'src/features/ipfs/ipfsSettings/ipfsComponents/infoIpfsNode';
import { selectUnreadCounts } from 'src/features/sense/redux/sense.redux';
import { useGetBalance } from 'src/pages/robot/_refactor/account/hooks';
import { useAppSelector } from 'src/redux/hooks';
import { RootState } from 'src/redux/store';
import { getFollowers, getTransactions, getTweet } from 'src/services/transactions/lcd';
import { getIpfsHash } from 'src/utils/ipfs/helpers';
import { convertResources, reduceBalances } from 'src/utils/utils';
import { useRobotContext } from '../robot.context';

// remove
async function getCyberlinksTotal(address: string) {
  try {
    const response = await getTransactions({
      events: [
        { key: 'message.action', value: '/cyber.graph.v1beta1.MsgCyberlink' },
        { key: 'cyberlink.neuron', value: address },
      ],
      pagination: { limit: 5, offset: 0 },
    });

    return response?.pagination?.total;
  } catch (error) {
    console.log(error);
    return undefined;
  }
}

function useMenuCounts(address: string | null) {
  const [tweetsCount, setTweetsCount] = useState();
  const [cyberlinksCount, setCyberlinksCount] = useState();
  const [energy, setEnergy] = useState<number>();
  const [followers, setFollowers] = useState<number>();
  const [sequence, setSequence] = useState<number>();

  const location = useLocation();
  const { addRefetch, isOwner } = useRobotContext();

  const { total: unreadSenseTotal } = useAppSelector(selectUnreadCounts);

  async function getTweetCount() {
    try {
      const response = await getTweet(address);
      setTweetsCount(response?.pagination?.total || 0);
    } catch (error) {
      console.error(error);
    }
  }

  useEffect(() => {
    if (!address) {
      return;
    }
    const type = location.pathname.split('/')[2];

    if (type === 'log') {
      addRefetch(getTweetCount);
    }
  }, [location.pathname, address, addRefetch, getTweetCount]);

  const { accounts } = useSelector((state: RootState) => state.pocket);

  const queryClient = useQueryClient();

  // const { staking } = useGetHeroes(address);
  const { totalAmountInLiquid } = useGetBalanceBostrom(address);
  const { repoSizeValue } = useGetIpfsInfo();
  const { balance } = useGetBalance(address);

  const { data: karma } = useGetKarma(address);

  const { resultGol } = useGetGol(address);
  const badges = Object.keys(resultGol).length ? Object.keys(resultGol).length - 1 : 0;

  async function getCyberlinksCount() {
    try {
      const response = await getCyberlinksTotal(address);
      setCyberlinksCount(response);
    } catch (error) {
      console.error(error);
    }
  }

  async function getSequence() {
    try {
      const response = await queryClient?.getSequence(address);
      setSequence(response?.sequence || 0);
    } catch (error) {
      console.error(error);
    }
  }

  async function getEnergy() {
    try {
      const allBalances = await queryClient?.getAllBalances(address);
      const balances = reduceBalances(allBalances);

      if (balances.milliampere && balances.millivolt) {
        const { milliampere, millivolt } = balances;
        setEnergy(convertResources(milliampere) * convertResources(millivolt));
      }
    } catch (error) {
      console.error(error);
    }
  }

  async function getFollow() {
    try {
      const addressHash = await getIpfsHash(address);
      const response = await getFollowers(addressHash);

      if (response?.pagination?.total) {
        setFollowers(response.pagination.total);
      }
    } catch (error) {
      console.error(error);
    }
  }

  useEffect(() => {
    if (!address) {
      return;
    }

    getCyberlinksCount();
    getFollow();
    getSequence();
    getTweetCount();
    getEnergy();
  }, [address, getCyberlinksCount, getEnergy, getFollow, getSequence, getTweetCount]);

  return {
    log: tweetsCount,
    // security: Object.keys(staking).length,
    badges,
    swarm: followers,
    energy: energy || 0,
    sigma: Number(totalAmountInLiquid.currentCap || 0),
    cyberlinks: cyberlinksCount,
    passport: accounts ? Object.keys(accounts).length : 0,
    drive: (isOwner && repoSizeValue) || '-',
    sense: isOwner ? unreadSenseTotal : 0, // 0 is temp
    karma: karma || 0,
    txs: sequence,
    rewards: balance.rewards,
  };
}

export default useMenuCounts;

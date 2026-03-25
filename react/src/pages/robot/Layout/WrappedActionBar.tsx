import { useCallback, useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { ActionBar } from 'src/components';
import ActionBarContainer from 'src/pages/robot/_refactor/account/actionBar';
import { useAppSelector } from 'src/redux/hooks';
import { checkFollow } from 'src/services/transactions/lcd';
import { getIpfsHash } from 'src/utils/ipfs/helpers';
import { useRobotContext } from '../robot.context';

function WrappedActionBar() {
  const { defaultAccount } = useAppSelector((state) => state.pocket);
  const activeAddress = defaultAccount.account?.cyber.bech32;
  const params = useParams();
  const tab = params['*'];

  const { address, refetchData } = useRobotContext();

  const [tweets, setTweets] = useState(false);
  const [follow, setFollow] = useState(false);

  const checkFollowAddress = useCallback(async () => {
    if (!address || !activeAddress || activeAddress === address) {
      return;
    }

    const addressFromIpfs = await getIpfsHash(address);
    const response = await checkFollow(activeAddress, addressFromIpfs);

    if (response && Number(response.txResponses.length) === 0) {
      setFollow(true);
      // setTweets(false);
    } else {
      setFollow(false);
    }
  }, [activeAddress, address]);

  useEffect(() => {
    checkFollowAddress();
  }, [checkFollowAddress]);

  useEffect(() => {
    if (defaultAccount.account?.cyber.keys === 'read-only') {
      return;
    }

    if (address === activeAddress) {
      setTweets(true);
    } else {
      setTweets(false);
    }

    setFollow(false);
  }, [defaultAccount, address, activeAddress]);

  return (
    <div
      style={{
        left: 0,
        position: 'fixed',
      }}
    >
      {activeAddress ? (
        <ActionBarContainer
          updateAddress={() => {
            refetchData();

            if (follow) {
              checkFollowAddress();
            }
          }}
          addressSend={address}
          type={tab}
          follow={follow}
          tweets={tweets}
          defaultAccount={defaultAccount.account?.cyber}
        />
      ) : (
        <ActionBar />
      )}
    </div>
  );
}

export default WrappedActionBar;

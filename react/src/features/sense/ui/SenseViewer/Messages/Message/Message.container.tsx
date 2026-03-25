import { useEffect, useState } from 'react';
import { Account, Dots } from 'src/components';

import ContentIpfs from 'src/components/contentIpfs/contentIpfs';
import { PATTERN_IPFS_HASH } from 'src/constants/patterns';
import { SenseItem } from 'src/features/sense/redux/sense.redux';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppSelector } from 'src/redux/hooks';
import { getIpfsHash } from 'src/utils/ipfs/helpers';
import useParticleDetails from '../../../../../particle/useParticleDetails';
import { formatSenseItemDataToUI } from '../../../utils/format';
import Message from './Message';
import styles from './Message.container.module.scss';

type Props = {
  senseItem: SenseItem;
};

function MessageContainer({ senseItem, currentChatId }: Props) {
  const [textCid, setTextCid] = useState<string>();

  const address = useAppSelector(selectCurrentAddress);

  const {
    timestamp,
    transactionHash,
    amount,
    from,
    cid,
    text,
    fromLog,
    isAmountSendToMyAddress,
    isFollow,
  } = formatSenseItemDataToUI(senseItem, address, currentChatId);

  const particleDetails = useParticleDetails(cid!, {
    skip: Boolean(text && !cid),
  });

  useEffect(() => {
    if (!text || text.match(PATTERN_IPFS_HASH)) {
      return;
    }

    (async () => {
      const cid = await getIpfsHash(text);
      setTextCid(cid);
    })();
  }, [text]);

  let content;

  if (cid) {
    const { data, loading } = particleDetails;

    if (loading) {
      content = (
        <span>
          resolving particle <Dots />
        </span>
      );
    } else if (data) {
      if (isFollow) {
        // some CIDs are not addresses
        // /sense/@master
        const address = data.content?.includes('bostrom') && data.content;

        content = (
          <div className={styles.follow}>
            <span>💚</span> {address && <Account address={address} avatar sizeAvatar={20} />}
          </div>
        );
      } else {
        content = <ContentIpfs details={data} cid={data.cid} content={data.content} search />;
      }
    }
  } else if (text) {
    content = text;
  }

  return (
    <Message
      address={from}
      myMessage={address === from}
      fromLog={fromLog}
      from={currentChatId !== from ? from : undefined}
      transactionHash={transactionHash}
      date={timestamp}
      content={content}
      cid={cid || textCid}
      amountData={{
        amount,
        isAmountSendToMyAddress,
      }}
      status={senseItem.status}
    />
  );
}

export default MessageContainer;

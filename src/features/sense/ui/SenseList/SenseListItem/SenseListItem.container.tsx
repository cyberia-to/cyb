import { Dots } from 'src/components';
import { contentTypeConfig } from 'src/containers/Search/Filters/Filters';
import { isParticle } from 'src/features/particle/utils';
import { SenseItemId } from 'src/features/sense/types/sense';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppSelector } from 'src/redux/hooks';
import useParticleDetails from '../../../../particle/useParticleDetails';
import { formatSenseItemDataToUI } from '../../utils/format';
import SenseListItem from './SenseListItem';

type Props = {
  senseItemId: SenseItemId;
};

function SenseListItemContainer({ senseItemId }: Props) {
  const { senseData, unreadCount } = useAppSelector((store) => {
    const chat = store.sense.chats[senseItemId]!;

    const lastMsg = chat.data[chat.data.length - 1];

    return {
      senseData: lastMsg,
      unreadCount: chat.unreadCount,
    };
  });
  const address = useAppSelector(selectCurrentAddress);

  const { timestamp, amount, cid, text, isAmountSendToMyAddress, isFollow, from } =
    formatSenseItemDataToUI(senseData, address, senseItemId);

  const particle = isParticle(senseItemId);

  const chatParticleDetails = useParticleDetails(senseItemId, {
    skip: !particle,
  });

  const { data, loading } = useParticleDetails(cid!, {
    skip: Boolean(text && !cid),
  });

  let content;

  if (cid) {
    if (loading) {
      content = (
        <span>
          resolving particle <Dots />
        </span>
      );
    } else if (data) {
      if (isFollow) {
        content = (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 5,
            }}
          >
            <span
              style={{
                position: 'relative',
                top: 2,
              }}
            >
              💚
            </span>{' '}
            {/* <Account address={data.content} avatar sizeAvatar={20} />{' '} */}
            neuron
          </div>
        );
      } else {
        content =
          data.text?.replaceAll('#', '') ||
          contentTypeConfig[data.type]?.label ||
          'unsupported type';
      }
    }
  } else {
    content = text;
  }

  function formatParticleTitle(text?: string, type: string) {
    if (type === 'image' || !text) {
      return null;
    }

    return text.trim().substring(0, 30).replaceAll('#', '');
  }

  const { text: particleText } = chatParticleDetails.data || {};

  return (
    <SenseListItem
      address={senseItemId}
      date={timestamp}
      title={formatParticleTitle(particleText, data?.type)}
      unreadCount={unreadCount}
      content={content}
      from={from}
      status={senseData.status}
      amountData={{ amount, isAmountSendToMyAddress }}
    />
  );
}

export default SenseListItemContainer;

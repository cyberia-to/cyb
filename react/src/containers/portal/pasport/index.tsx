import { useEffect, useMemo, useState } from 'react';
import { PATTERN_CYBER } from 'src/constants/patterns';
import { useQueryClient } from 'src/contexts/queryClient';
import { Nullable } from 'src/types';
import { Citizenship } from 'src/types/citizenship';
import { v4 as uuidv4 } from 'uuid';
import { ContainerGradient } from '../../../components';
import plus from '../../../image/plus.svg';
import { formatNumber, trimString } from '../../../utils/utils';
import { MusicalAddress, ParseAddressesImg } from '../components';
import { AvataImgIpfs } from '../components/avataIpfs';
import ContainerAvatar from '../components/avataIpfs/containerAvatar';
import BtnPasport from './btnPasport';

// generated, check
type Props = {
  title: string;
  initState: any; // Replace 'any' with the actual type
  stateOpen: any; // Replace 'any' with the actual type
  citizenship: Nullable<Citizenship>;
  karma: number;
  onClickEditAvatar?: () => void;
  addressActiveSignatures: any | null; // Replace 'any' with the actual type
};

function PasportCitizenship({
  citizenship,
  txHash,
  updateFunc,
  stateOpen,
  initStateCard,
  setActiveItem,
  totalGift,
  onClickDeleteAddress,
  onClickProveeAddress,
  onClickEditAvatar,
}: Props) {
  const queryClient = useQueryClient();
  const [owner, setOwner] = useState(null);
  const [addresses, setAddresses] = useState(null);
  const [active, setActive] = useState(0);
  const [karma, setKarma] = useState(0);

  useEffect(() => {
    if (setActiveItem !== undefined) {
      setActive(setActiveItem);
    }
  }, [setActiveItem]);

  useEffect(() => {
    if (updateFunc) {
      if (addresses !== null && Object.hasOwn(addresses, active)) {
        updateFunc(addresses[active].address);
      } else {
        updateFunc(null);
      }
    }
  }, [addresses, active, updateFunc]);

  // const useGetOwner

  useEffect(() => {
    const getKarma = async () => {
      try {
        if (owner?.match(PATTERN_CYBER) && queryClient) {
          const responseKarma = await queryClient.karma(owner);
          if (responseKarma.karma) {
            setKarma(parseFloat(responseKarma.karma));
          }
        }
      } catch (error) {
        console.log('error', error);
      }
    };
    getKarma();
  }, [owner, queryClient]);

  useEffect(() => {
    const getPasport = async () => {
      if (citizenship) {
        setOwner(citizenship.owner);
        const addressesData = [];
        if (citizenship.extension.addresses && citizenship.extension.addresses.length > 0) {
          addressesData.push(...citizenship.extension.addresses);
        }
        setAddresses([{ address: citizenship.owner }, ...addressesData]);
      }
    };
    getPasport();
  }, [citizenship]);

  const addressActiveSignatures = useMemo(() => {
    if (addresses !== null) {
      if (Object.hasOwn(addresses, active)) {
        return { bech32: addresses[active].address };
      }
      setActive(0);
      return { bech32: addresses[0].address };
    }

    return null;
  }, [addresses, active]);

  const useClosedTitle = useMemo(() => {
    if (citizenship) {
      return (
        <div
          style={{
            display: 'flex',
            width: '100%',
            justifyContent: 'space-between',
            zIndex: '1',
            alignItems: 'center',
            height: 32,
          }}
        >
          <div style={{ color: '#1fcbff' }}>{citizenship?.extension.nickname}</div>

          <div style={{ color: '#36D6AE' }}>
            {addresses !== null && Object.hasOwn(addresses, active) && (
              <div style={{ display: 'flex', alignItems: 'center', height: 25 }}>
                {trimString(addresses[active].address, 8, 4)}
                <ParseAddressesImg key={addresses[active].address} address={addresses[active]} />
              </div>
            )}
          </div>

          <div style={{ width: '32px', height: '32px' }}>
            <AvataImgIpfs cidAvatar={citizenship.extension.avatar} />
          </div>
        </div>
      );
    }
    return null;
  }, [citizenship, addresses, active]);

  const checkClaimedAddress = (itemAddress, totalGiftArr) => {
    const statusAddress = {
      gift: false,
      claimed: false,
    };

    if (totalGiftArr && totalGiftArr !== null && Object.hasOwn(totalGiftArr, itemAddress)) {
      statusAddress.gift = true;
      const giftByAddress = totalGiftArr[itemAddress];
      if (giftByAddress.isClaimed) {
        statusAddress.claimed = true;
      }
    }

    return statusAddress;
  };

  const renderItemImg = useMemo(() => {
    if (addresses !== null) {
      return addresses.map((item, index) => {
        const key = uuidv4();
        const statusAddressGiftData = checkClaimedAddress(item.address, totalGift);

        return (
          <ParseAddressesImg
            key={key}
            address={item}
            active={index === active}
            onClick={() => setActive(index)}
            statusAddressGift={statusAddressGiftData}
          />
        );
      });
    }

    return [];
  }, [active, addresses, totalGift, checkClaimedAddress]);

  return (
    <ContainerGradient
      txs={txHash}
      closedTitle={useClosedTitle}
      title="Moon Citizenship"
      initState={initStateCard}
      stateOpen={stateOpen}
    >
      <div
        style={{
          height: '100%',
          color: '#36D6AE',
          display: 'grid',
          gap: '15px 0',
        }}
      >
        <div
          style={{
            display: 'grid',
            // height: '60px',
            gap: '20px',
          }}
        >
          <div style={{ color: '#36D6AE', lineHeight: '18px' }}>
            {citizenship?.extension.nickname}
          </div>
          <div style={{ lineHeight: '18px' }}>karma {karma > 0 ? formatNumber(karma) : ''} 🔮</div>
          <ContainerAvatar>
            <AvataImgIpfs cidAvatar={citizenship?.extension.avatar || false} />
            {onClickEditAvatar && (
              <BtnPasport
                onClick={onClickEditAvatar}
                typeBtn="blue"
                style={{
                  position: 'absolute',
                  left: '100%',
                }}
              >
                edit
              </BtnPasport>
            )}
          </ContainerAvatar>
        </div>
        {addressActiveSignatures !== null && (
          <div
            style={{
              // height: 'calc(100% - 50px)',
              display: 'grid',
              gap: '20px 0',
            }}
          >
            <div style={{ position: 'relative' }}>
              <div
                style={{
                  display: 'grid',
                  gap: '15.5px',
                  gridTemplateColumns: 'repeat(9, 30px)',
                  width: '100%',
                }}
              >
                {renderItemImg}
              </div>
              {active === 0 && onClickProveeAddress && (
                <BtnPasport
                  onClick={onClickProveeAddress}
                  typeBtn="blue"
                  style={{
                    position: 'absolute',
                    left: '100%',
                    top: 0,
                  }}
                >
                  <img style={{ width: 13, height: 13 }} src={plus} alt="plus" />
                </BtnPasport>
              )}
              {active !== 0 && onClickDeleteAddress && (
                <BtnPasport
                  onClick={onClickDeleteAddress}
                  typeBtn="red"
                  style={{
                    position: 'absolute',
                    left: '100%',
                    top: 0,
                  }}
                >
                  X
                </BtnPasport>
              )}
            </div>
            <MusicalAddress address={addressActiveSignatures.bech32} />
          </div>
        )}
      </div>
    </ContainerGradient>
  );
}

export default PasportCitizenship;

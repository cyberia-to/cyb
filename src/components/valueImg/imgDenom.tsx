import tocyb from 'images/boot.png';
import cosmos from 'images/cosmos.svg';

import eth from 'images/Ethereum_logo_2014.svg';
import pool from 'images/gravitydexPool.png';
import hydrogen from 'images/hydrogen.svg';
import ibc from 'images/ibc-unauth.png';
import boot from 'images/large-green.png';
import defaultImg from 'images/large-orange-circle.png';
import amperImg from 'images/light.png';
import voltImg from 'images/lightning2.png';
import osmosis from 'images/osmosis.svg';
import pussy from 'images/space-pussy.svg';
import { useEffect, useState } from 'react';
import { checkIsEmoji } from 'src/utils/emoji';
import { trimString } from '../../utils/utils';
import Tooltip from '../tooltip/tooltip';
import lp from './images/lp.png';
import styles from './TextDenom.module.scss';

// maybe reuse enum from DenomArr
const nativeImageMap = {
  millivolt: voltImg,
  v: voltImg,
  milliampere: amperImg,
  a: amperImg,
  hydrogen,
  h: hydrogen,
  liquidpussy: lp,
  lp,
  boot,
  pussy,
  tocyb,
  eth,
  osmo: osmosis,
  uosmo: osmosis,
  atom: cosmos,
  uatom: cosmos,
  cosmosvaloper: cosmos,
};

const getNativeImg = (text: string) => {
  return nativeImageMap[text.toLowerCase()] || defaultImg;
};

type ImgDenomProps = {
  coinDenom: string;
  marginImg?: number;
  size?: number;
  zIndexImg?: number;
  tooltipStatus: boolean;
  infoDenom?: {
    coinImageCid?: string;
    path?: string;
    native?: boolean;
    denom?: string;
  };
};

const PINATA_GATEWAY = 'https://bostrom.mypinata.cloud';
const CYBER_GW = 'https://gateway.ipfs.cybernode.ai';

const cidToGatewayUrl = (cid: string) => `${PINATA_GATEWAY}/ipfs/${cid}`;
const cidToFallbackUrl = (cid: string) => `${CYBER_GW}/ipfs/${cid}`;

function ImgDenom({
  coinDenom,
  marginImg,
  size,
  zIndexImg,
  tooltipStatus,
  infoDenom,
}: ImgDenomProps) {
  const [imgDenom, setImgDenom] = useState<string>();
  const [tooltipText, setTooltipText] = useState<string>(coinDenom);

  useEffect(() => {
    if (infoDenom && Object.hasOwn(infoDenom, 'coinImageCid')) {
      const { coinImageCid, path, native } = infoDenom;
      if (coinImageCid) {
        // Use Pinata gateway directly for pinned icons
        setImgDenom(cidToGatewayUrl(coinImageCid));
      } else if (native) {
        if (coinDenom.includes('pool')) {
          setImgDenom(pool);
          setTooltipText(trimString(coinDenom, 9, 9));
        } else {
          if (infoDenom.denom) {
            setTooltipText(infoDenom.denom);
          }

          const nativeImg = getNativeImg(coinDenom) || getNativeImg(infoDenom.denom || '');
          setImgDenom(nativeImg);
        }
      } else {
        const denomKey = (infoDenom.denom || coinDenom).toLowerCase();
        const knownImg = nativeImageMap[denomKey];
        setImgDenom(knownImg || ibc);
      }

      if (path && path.length > 0) {
        setTooltipText(path);
      }
    } else {
      setImgDenom(getNativeImg(coinDenom));
    }
  }, [coinDenom, infoDenom]);

  const isEmoji = imgDenom && checkIsEmoji(imgDenom);

  const img = isEmoji ? (
    imgDenom
  ) : (
    <img
      style={{
        margin: marginImg || 0,
        width: size || 20,
        height: size || 20,
        zIndex: zIndexImg || 0,
        verticalAlign: 'middle',
      }}
      src={imgDenom || defaultImg}
      alt="text"
      onError={(e) => {
        const img = e.currentTarget;
        const src = img.src;
        // Fallback: Pinata → cybernode → default
        if (src.includes(PINATA_GATEWAY)) {
          const cid = src.split('/ipfs/')[1];
          if (cid) {
            img.src = cidToFallbackUrl(cid);
            return;
          }
        }
        img.src = defaultImg;
      }}
    />
  );

  if (tooltipStatus) {
    return (
      <div>
        <Tooltip
          placement="top"
          contentStyle={{ display: 'flex' }}
          tooltip={<div className={styles.denom}>{tooltipText}</div>}
        >
          {img}
        </Tooltip>
      </div>
    );
  }

  return <div style={{ display: 'flex' }}>{img}</div>;
}

export default ImgDenom;

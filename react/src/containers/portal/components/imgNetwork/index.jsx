import {
  PATTERN_COSMOS,
  PATTERN_CYBER,
  PATTERN_ETH,
  PATTERN_SPACE_PUSSY,
} from 'src/constants/patterns';
import imgSpacePussy from '../../../../image/space-pussy.svg';

import { GIFT_ICON } from '../../utils';
import styles from './styles.scss';

const classNames = require('classnames');

const imgEth = require('../../../../image/Ethereum_logo_2014.svg');
const imgBostrom = require('../../../../image/large-green.png');
const imgCosmos = require('../../../../image/cosmos-2.svg');
const imgOsmosis = require('../../../../image/osmosis.svg');
const imgTerra = require('../../../../image/terra.svg');

const defaultImg = require('../../../../image/large-yellow-circle.png');
const imgHome = require('../../../../image/home-icon.png');

const PATTERN_OSMOSIS = /^osmo[a-zA-Z0-9]{39}$/g;
const PATTERN_TERRA = /^terra[a-zA-Z0-9]{39}$/g;

const GIFT_ICON_CLAIM = '🎁✅';

function ParseAddressesImg({ address, active, statusAddressGift, ...props }) {
  let gift;
  let claimed;

  if (statusAddressGift) {
    gift = statusAddressGift.gift;
    claimed = statusAddressGift.claimed;
  }

  let img = defaultImg;

  if (address.address.match(PATTERN_ETH)) {
    img = imgEth;
  }

  if (address.address.match(PATTERN_COSMOS)) {
    img = imgCosmos;
  }

  if (address.address.match(PATTERN_OSMOSIS)) {
    img = imgOsmosis;
  }

  if (address.address.match(PATTERN_TERRA)) {
    img = imgTerra;
  }

  if (address.address.match(PATTERN_CYBER)) {
    img = imgBostrom;
  }

  if (address.address.match(PATTERN_SPACE_PUSSY)) {
    img = imgSpacePussy;
  }

  return (
    <button className={styles.ButtonIconNetwork} type="button" {...props}>
      <img src={img} alt="img" />
      {address.address.match(PATTERN_CYBER) && (
        <img className={styles.IconHome} src={imgHome} alt="img" />
      )}
      {gift && claimed === false && (
        <div
          style={{
            position: 'absolute',
            left: '50%',
            marginRight: '-50%',
            transform: 'translate(-50%, -16px)',
          }}
        >
          {GIFT_ICON}
        </div>
      )}
      {claimed && (
        <div
          style={{
            position: 'absolute',
            left: '50%',
            marginRight: '-50%',
            width: '35px',
            transform: 'translate(-50%, -16px)',
          }}
        >
          {GIFT_ICON_CLAIM}
        </div>
      )}
      {active && (
        <>
          <div className={classNames(styles.textboxFace, styles.textboxBottomGradient)} />
          <div className={classNames(styles.textboxBottomLine)} />
        </>
      )}
    </button>
  );
}

export default ParseAddressesImg;

const voltImg = require('../../../image/lightning2.png');
const amperImg = require('../../../image/light.png');
const hydrogen = require('../../../image/hydrogen.svg');

function ValueImg({ text, onlyImg }) {
  let img = null;
  let textCurency = text;

  switch (text) {
    case 'millivolt':
      img = voltImg;
      textCurency = 'V';
      break;

    case 'milliampere':
      img = amperImg;
      textCurency = 'A';
      break;

    case 'hydrogen':
      img = hydrogen;
      textCurency = 'H';
      break;

    default:
      if (text.length > 6) {
        textCurency = text.slice(6);
        img = null;
        break;
      } else {
        textCurency = text;
        img = null;
        break;
      }
  }

  return (
    <div style={{ display: 'flex' }}>
      {!onlyImg && <span>{textCurency.toUpperCase()}</span>}
      {img !== null && (
        <img
          style={{ marginLeft: 5, width: 20, height: 20, objectFit: 'cover' }}
          src={img}
          alt="text"
        />
      )}
    </div>
  );
}

export default ValueImg;

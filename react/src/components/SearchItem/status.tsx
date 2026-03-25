import cx from 'classnames';
import Tooltip from '../tooltip/tooltip';
import styles from './styles.module.scss';

const size = '15 Mb';

export type StatusType =
  | 'executing'
  | 'timeout'
  | 'availableDownload'
  | 'completed'
  | 'sparkApp'
  | 'legacy';

const typeStatus = (type: StatusType) => {
  let status = {
    color: '#546e7a',
    text: 'IPFS',
  };

  switch (type) {
    case 'executing':
      status = {
        color: '#00b0ff',
        text: 'cyb: I am getting content meta information',
      };
      break;
    case 'timeout':
      status = {
        color: '#ff3d00',
        text: 'cyb: I was unable to find traces of this in dht :-(',
      };
      break;
    case 'availableDownload':
      status = {
        color: '#ffd600',
        text: `cyb: ${size} answer is available for download`,
      };
      break;
    case 'completed':
      status = {
        color: '#00e676',
        text: `cyb: ${size} answer is loaded`,
      };
      break;
    case 'sparkApp':
      status = {
        color: '#d500f9',
        text: 'cyb: Answer is loaded instantly through spark app',
      };
      break;
    case 'legacy':
      status = {
        color: '#ff9100',
        text: 'cyb: I can not load content from legacy web. You have to go and do it yourself. But I would recommend to ~transform it',
      };
      break;
    default:
      status = {
        color: '#546e7a',
        text: 'IPFS',
      };
      break;
  }
  return status;
};

type Props = {
  status: StatusType;
};

function Status({ status }: Props) {
  const { color, text } = typeStatus(status);

  return (
    <div>
      <Tooltip placement="bottom" tooltip={text}>
        <div
          className={cx(styles.customPill, {
            [styles.glow]: status === 'executing',
          })}
          style={{ backgroundColor: color }}
        />
      </Tooltip>
    </div>
  );
}

export default Status;

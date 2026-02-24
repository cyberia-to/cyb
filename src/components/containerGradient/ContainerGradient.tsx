// TODO: need use Display component for this
import React, { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { Transition } from 'react-transition-group';
import { trimString } from '../../utils/utils';
import styles from './ContainerGradient.module.scss';
import Display from './Display/Display';
import DisplayTitle from './DisplayTitle/DisplayTitle';
import { ColorLamp } from './types';

const classNames = require('classnames');

function ContainerLamp({ style, children }) {
  return (
    <div
      className={classNames(styles.wrapContainerLamp, {
        [styles.wrapContainerLampGreen]: style === 'green',
        [styles.wrapContainerLampBlue]: style === 'blue' || style === 'grey',
        [styles.wrapContainerLampRed]: style === 'red',
        [styles.wrapContainerLampYellow]: style === 'yellow',
        [styles.wrapContainerLampPurple]: style === 'purple',
        [styles.wrapContainerLampPink]: style === 'pink',
        [styles.wrapContainerLampDefault]: !style,
      })}
    >
      {children}
    </div>
  );
}

type Txs = {
  status: 'pending' | 'confirmed' | 'error';
  txHash?: string;
  rawLog?: string;
};

type TxsProps = {
  data: Txs;
};

// TODO: move to action bar
function TxsStatus({ data }: TxsProps) {
  let style;
  switch (data.status) {
    case 'pending':
      style = 'yellow';
      break;
    case 'confirmed':
      style = 'green';
      break;
    case 'error':
      style = 'red';
      break;

    default:
      break;
  }
  return (
    <ContainerLamp style={style}>
      <div
        className={classNames(styles.containerTxs, {
          [styles.containerTxsPending]: data.status === 'pending',
          [styles.containerTxsConfirmed]: data.status === 'confirmed',
          [styles.containerTxsDanger]: data.status === 'error',
        })}
      >
        {data.txHash && (
          <div className={styles.containerTxsTxHash}>
            <Link to={`/network/bostrom/tx/${data.txHash}`}>
              <div>{trimString(data.txHash, 5, 5)}</div>
            </Link>
            <div>{data.status}</div>
          </div>
        )}
        {data.rawLog && <div className={styles.containerTxsRawLog}>{data.rawLog}</div>}
      </div>
    </ContainerLamp>
  );
}

interface Props {
  title?: string;
  closedTitle?: string;

  children?: React.ReactNode;
  txs?: Txs;
  userStyleContent?: React.CSSProperties;
  stateOpen?: boolean;
  initState?: boolean;
  styleLampContent?: ColorLamp;
  styleLampTitle?: ColorLamp;
  togglingDisable?: boolean;
}

function ContainerGradient({
  title = 'Moon Citizenship',
  closedTitle,
  children,
  txs,
  userStyleContent,
  stateOpen,
  initState = true,
  styleLampContent = 'blue',
  styleLampTitle,
  togglingDisable,
}: Props) {
  const [isOpen, setIsOpen] = useState(initState);

  useEffect(() => {
    if (stateOpen !== undefined) {
      setIsOpen(stateOpen);
    }
  }, [stateOpen]);

  const toggling = () => {
    if (togglingDisable === undefined || togglingDisable === false) {
      return setIsOpen(!isOpen);
    }
    return undefined;
  };

  const useTitle = useCallback(
    (state) => {
      if (!isOpen && closedTitle && closedTitle !== null && state === 'exited') {
        return closedTitle;
      }
      if (state === 'entered') {
        return title;
      }

      return title;
    },
    [isOpen, closedTitle, title]
  );

  return (
    <div>
      {/* TODO: use Display component */}
      <div
        className={classNames(styles.containerContainerGradient, {
          [styles.togglingDisable]: togglingDisable,
          [styles.containerContainerGradientPrimary]: !styleLampContent,
          [styles.containerContainerGradientPrimary]: styleLampContent === 'blue',
          [styles.containerContainerGradientDanger]: styleLampContent === 'red',
          [styles.containerContainerGradientGreen]: styleLampContent === 'green',
        })}
      >
        <Transition in={isOpen} timeout={500}>
          {(state) => {
            return (
              <>
                <button
                  type="button"
                  onClick={!togglingDisable ? toggling : undefined}
                  role={!togglingDisable ? 'presentation' : undefined}
                  style={{
                    position: 'relative',
                    left: -2,
                    width: '100%',
                    color: 'inherit',
                  }}
                  className={classNames({
                    [styles.titleTogglingActive]: !togglingDisable,
                  })}
                >
                  <DisplayTitle
                    title={useTitle(state)}
                    animationState={state}
                    color={styleLampTitle}
                  />
                </button>

                <div
                  style={userStyleContent}
                  className={classNames(
                    styles.containerContainerGradientContent,
                    {
                      [styles.containerContainerGradientContentPrimary]: !styleLampContent,
                      [styles.containerContainerGradientContentPrimary]:
                        styleLampContent === 'blue',
                      [styles.containerContainerGradientContentDanger]: styleLampContent === 'red',
                      [styles.containerContainerGradientContentGreen]: styleLampContent === 'green',
                    },
                    styles[`containerContainerGradientContent${state}`]
                  )}
                >
                  {children}
                </div>
              </>
            );
          }}
        </Transition>
      </div>
      {txs && <TxsStatus data={txs} />}
    </div>
  );
}

export const ContainerGradientText = Display;

export default ContainerGradient;

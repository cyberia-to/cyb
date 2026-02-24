import { useEffect, useState } from 'react';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { TracesDenomFuncResponse } from 'src/types/ibc';
import Denom from './index';

// generated, recheck
type PossibleTokens =
  | 'millivolt'
  | 'milliampere'
  | 'hydrogen'
  | 'h'
  | 'liquidpussy'
  | 'lp'
  | 'boot'
  | 'pussy'
  | 'tocyb'
  | 'eth'
  | 'bostrom'
  | 'space-pussy'
  | 'cosmos'
  | 'osmo'
  | 'terra';

type DenomArrProps = {
  denomValue: PossibleTokens | any;
  onlyText?: boolean;
  onlyImg?: boolean;
  tooltipStatusImg?: boolean;
  tooltipStatusText?: boolean;
  type?: 'network';
  size?: number;
};

function DenomArr({
  denomValue,
  onlyText,
  onlyImg,
  tooltipStatusImg,
  tooltipStatusText,
  type,
  size,
}: DenomArrProps) {
  const { tracesDenom } = useIbcDenom();

  const [denom, setDenom] = useState<TracesDenomFuncResponse[]>();

  useEffect(() => {
    if (type) {
      return;
    }
    const denomTraces = tracesDenom(denomValue);

    setDenom(denomTraces);
  }, [denomValue, tracesDenom, type]);

  if (type === 'network') {
    return (
      <Denom
        onlyImg={onlyImg}
        onlyText={onlyText}
        denomValue={denomValue}
        tooltipStatusImg={tooltipStatusImg}
        tooltipStatusText={tooltipStatusText}
        type={type}
        infoDenom={{}}
        gap={13}
        size={size}
      />
    );
  }

  if (denom && denom.length === 1) {
    return (
      <Denom
        onlyImg={onlyImg}
        onlyText={onlyText}
        denomValue={denom[0].denom}
        tooltipStatusImg={tooltipStatusImg}
        tooltipStatusText={tooltipStatusText}
        type={type}
        infoDenom={denom[0]}
        gap={13}
        size={size}
      />
    );
  }

  if (denom && denom.length > 1) {
    return (
      <div style={{ display: 'flex' }}>
        {!onlyText && (
          <>
            <Denom
              type={type}
              denomValue={denom[0].denom}
              onlyImg
              tooltipStatusImg={tooltipStatusImg}
              tooltipStatusText={tooltipStatusText}
              infoDenom={denom[0]}
              size={size}
            />
            <Denom
              type={type}
              denomValue={denom[1].denom}
              marginContainer={onlyImg ? '0px 0px 0px -12px' : '0px 5px 0px -12px'}
              onlyImg
              tooltipStatusImg={tooltipStatusImg}
              tooltipStatusText={tooltipStatusText}
              infoDenom={denom[1]}
              size={size}
            />
          </>
        )}

        {!onlyImg && (
          <>
            <Denom
              type={type}
              denomValue={denom[0].denom}
              onlyText
              tooltipStatusImg={tooltipStatusImg}
              tooltipStatusText={tooltipStatusText}
              infoDenom={denom[0]}
              size={size}
            />
            -
            <Denom
              type={type}
              denomValue={denom[1].denom}
              onlyText
              tooltipStatusImg={tooltipStatusImg}
              tooltipStatusText={tooltipStatusText}
              infoDenom={denom[1]}
              size={size}
            />
          </>
        )}
      </div>
    );
  }
  return null;
}

export default DenomArr;

/* eslint-disable no-nested-ternary */
/* eslint-disable jsx-a11y/click-events-have-key-events */
/* eslint-disable jsx-a11y/no-static-element-interactions */
import React from 'react';
import { DenomArr, OptionSelect, Select } from 'src/components';
import { $TsFixMeFunc } from 'src/types/tsfix';

type SelectTeleportProps = {
  type?: string;
  valueSelect: string;
  textSelectValue: string;
  onChangeSelect: $TsFixMeFunc;
  children: React.ReactNode;
  width?: string;
  custom?: boolean;
  disabled?: boolean;
};

function SelectTeleport({
  type,
  valueSelect,
  textSelectValue,
  onChangeSelect,
  children,
  width,
  custom,
  disabled,
}: SelectTeleportProps) {
  return (
    <Select
      valueSelect={valueSelect}
      disabled={disabled}
      width={width}
      onChangeSelect={onChangeSelect}
      currentValue={
        custom ? (
          <OptionSelect text={textSelectValue} value={valueSelect} />
        ) : valueSelect === '' ? (
          <OptionSelect text="choose" img={<DenomArr denomValue="choose" onlyImg />} value="" />
        ) : (
          <OptionSelect
            text={
              <DenomArr
                type={type}
                denomValue={textSelectValue}
                onlyText
                tooltipStatusText={false}
              />
            }
            bgrImg={textSelectValue.includes('pool')}
            img={
              <DenomArr type={type} denomValue={textSelectValue} onlyImg tooltipStatusImg={false} />
            }
            value={valueSelect}
          />
        )
      }
    >
      {children}
    </Select>
  );
}

export default SelectTeleport;

import { ReactNode } from 'react';
import TabItem, { Position } from './TabItem/TabItem';
import style from './Tabs.module.scss';

type optionsProps = {
  to?: string;
  onClick?: () => void;
  key: string;
  text?: ReactNode;
};

type Props = {
  options: optionsProps[];
  selected: string;
};

function Tabs({ options, selected }: Props) {
  return (
    <div className={style.wrapper}>
      {options.map((item, index) => {
        const type =
          index === 0 ? Position.Left : index === options.length - 1 ? Position.Right : undefined;

        const isSelected = selected === item.key;
        return (
          <TabItem
            key={index}
            type={type}
            isSelected={isSelected}
            to={item.to}
            onClick={item.onClick}
            text={item.text || item.key}
          />
        );
      })}
    </div>
  );
}

export default Tabs;

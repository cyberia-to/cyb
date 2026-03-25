import { Tablist } from '@cybercongress/gravity';
import { useNavigate } from 'react-router-dom';
import { TypeTab } from '../type';
import ButtonTeleport from './buttonGroup/indexBtn';

type TabListProps = {
  selected: TypeTab;
};

function TabList({ selected }: TabListProps) {
  const navigate = useNavigate();

  const handleHistory = (to: string) => {
    navigate(to);
  };

  return (
    <Tablist
      display="grid"
      gridTemplateColumns="repeat(auto-fit, minmax(110px, 1fr))"
      // gridGap="10px"
      marginBottom={30}
      maxWidth="390px"
      width="375px"
      marginX="auto"
    >
      <ButtonTeleport
        status="left"
        isSelected={selected === 'add-liquidity'}
        onClick={() => handleHistory('/warp/add-liquidity')}
      >
        Add liquidity
      </ButtonTeleport>
      <ButtonTeleport
        status="center"
        isSelected={selected === 'create-pool'}
        onClick={() => handleHistory('/warp/create-pool')}
      >
        Create pool
      </ButtonTeleport>
      <ButtonTeleport
        status="right"
        isSelected={selected === 'sub-liquidity'}
        onClick={() => handleHistory('/warp/sub-liquidity')}
      >
        Sub liquidity
      </ButtonTeleport>
    </Tablist>
  );
}

export default TabList;

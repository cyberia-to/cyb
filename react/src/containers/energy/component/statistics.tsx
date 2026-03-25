import { Pane } from '@cybercongress/gravity';
import { useNavigate } from 'react-router-dom';
import { formatNumber } from '../../../utils/utils';
import Card from '../ui/card';

type Props = {
  myEnergy: number;
  income: number;
  outcome: number;
  active?: string;
};

function Statistics({ myEnergy = 0, income = 0, outcome = 0, active }: Props) {
  const navigate = useNavigate();

  const freeEnergy = myEnergy + income - outcome;

  const onClickNavigate = (to?: string) => {
    if (!active) {
      navigate(`./${to || ''}`);
    } else {
      navigate(`../${to || ''}`, { relative: 'path' });
    }
  };

  return (
    <Pane
      marginTop={10}
      marginBottom={10}
      display="flex"
      alignItems="center"
      justifyContent="center"
      flex-irection="row"
    >
      <Card
        active={!active}
        title="Enegy"
        value={`${formatNumber(myEnergy)} W`}
        onClick={() => onClickNavigate()}
      />
      <Pane marginX={5} fontSize="20px">
        +
      </Pane>
      <Card
        active={active === 'income'}
        title="Income"
        value={`${formatNumber(income)} W`}
        onClick={() => onClickNavigate('income')}
      />
      <Pane marginX={5} fontSize="20px">
        -
      </Pane>
      <Card
        active={active === 'outcome'}
        title="Outcome"
        value={`${formatNumber(outcome)} W`}
        onClick={() => onClickNavigate('outcome')}
      />
      <Pane marginX={5} fontSize="20px">
        =
      </Pane>
      <Card
        title="Free Energy"
        value={`${formatNumber(freeEnergy > 0 ? freeEnergy : 0)} W`}

        // tooltipValue="Your rating in relation to the rating of the network is less than 1000%"
        // positionTooltip="bottom"
      />
    </Pane>
  );
}

export default Statistics;

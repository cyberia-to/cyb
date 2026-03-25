import Slider from 'rc-slider';
import 'rc-slider/assets/index.css';
import { GenericSliderProps } from 'rc-slider/lib/interface';

type Props = {
  minMax: {
    min: number;
    max: number;
  };
  value: {
    amount: number;
    onChange: (amount: number) => void;
  };
  marks: GenericSliderProps['marks'];
};

function RcSlider({ minMax, value, marks }: Props) {
  return (
    <Slider
      value={value.amount}
      min={minMax.min}
      max={minMax.max}
      marks={marks}
      onChange={value.onChange}
      trackStyle={{ backgroundColor: '#3ab793' }}
      railStyle={{ backgroundColor: '#97979775' }}
      dotStyle={{
        backgroundColor: '#97979775',
        borderColor: '#97979775',
      }}
      activeDotStyle={{
        borderColor: '#3ab793',
        backgroundColor: '#3ab793',
      }}
      handleStyle={{
        border: 'none',
        backgroundColor: '#3ab793',
      }}
    />
  );
}

export default RcSlider;

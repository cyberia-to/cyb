import { ComponentProps } from 'react';
import Button from '../btnGrd';

interface Props extends ComponentProps<typeof Button> {
  img: string;
}

function ButtonImgText({ img, text = 'Send', ...props }: Props) {
  return (
    <Button style={{ margin: '0 10px' }} {...props}>
      {text}{' '}
      {img && (
        <img
          style={{
            width: 20,
            height: 20,
            marginLeft: '5px',
            paddingTop: '2px',
            objectFit: 'contain',
          }}
          src={img}
          alt="img"
        />
      )}
    </Button>
  );
}

export default ButtonImgText;

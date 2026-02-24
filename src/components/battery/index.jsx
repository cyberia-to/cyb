import { Pane } from '@cybercongress/gravity';
import { useEffect, useState } from 'react';

function Battery({ percent = 0, widthBat = 60, borderSize = 2, heightBat = 20 }) {
  const [colorBar, setColorBar] = useState('#3ab793');

  useEffect(() => {
    switch (true) {
      case percent < 25:
        setColorBar('#ff3d00');
        break;
      case percent < 50:
        setColorBar('#ff6d00');
        break;
      case percent < 75:
        setColorBar('#ffff00');
        break;
      case percent > 75:
        setColorBar('#00e676');
        break;
      default:
        setColorBar('#3ab793');
        break;
    }
  }, [percent]);

  return (
    <Pane display="flex" alignItems="center">
      <Pane
        width={`${widthBat}px`}
        height={`${heightBat}px`}
        borderTop={`${borderSize}px solid ${colorBar}`}
        borderBottom={`${borderSize}px solid ${colorBar}`}
        borderLeft={`${borderSize}px solid ${colorBar}`}
        borderRight={`${borderSize}px solid ${colorBar}`}
        borderRadius="3px"
        paddingX="2px"
        paddingY="2px"
      >
        <Pane
          width={`${percent}%`}
          backgroundColor={colorBar}
          height="100%"
          borderRadius="2px"
          transition="1s"
        />
      </Pane>
      <Pane
        width="2px"
        height={`${heightBat * 0.4}px`}
        backgroundColor={colorBar}
        borderRadius="2px"
        marginLeft="1px"
      />
    </Pane>
  );
}

// eslint-disable-next-line import/no-unused-modules
export default Battery;

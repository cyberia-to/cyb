import React, { ComponentProps, CSSProperties, useCallback, useMemo } from 'react';
import { ReactReader } from 'react-reader';
import useEPubLocation from 'src/hooks/useEPubLocation';

interface IProps {
  url: string;
  search?: boolean;
  style?: CSSProperties;
}

type EpubInitOptions = ComponentProps<typeof ReactReader>['epubInitOptions'];
const epubInitOptions: EpubInitOptions = { openAs: 'epub' };

function EPubView({ url, search, style }: IProps) {
  const [location, setLocation] = useEPubLocation(url);
  const currentLocation = location && !search ? location : 0;

  const currentStyle = useMemo(
    () => ({ height: search ? '300px' : '60vh', ...style }),
    [style, search]
  );

  const onLocationChange = useCallback(
    (loc: string) => {
      if (!search) {
        setLocation(loc);
      }
    },
    [search, setLocation]
  );

  return (
    <div style={currentStyle}>
      <ReactReader
        url={url}
        location={currentLocation}
        locationChanged={onLocationChange}
        epubInitOptions={epubInitOptions}
      />
    </div>
  );
}

export default React.memo(EPubView);

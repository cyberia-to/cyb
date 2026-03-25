/* eslint-disable import/prefer-default-export */
import { useCallback } from 'react';
import type { EpubView } from 'react-reader';

const epubKey = 'cyb:epub';

const getEPubMap = (): Record<string, EpubView['location']> => {
  try {
    const epubMapString = localStorage.getItem(epubKey) || '';
    const epubMap = JSON.parse(epubMapString);

    return epubMap;
  } catch (error) {
    console.error('Failed to parse epub locations map:', error);

    return {};
  }
};

const getLocation = (url: string) => {
  const epubMap = getEPubMap();

  return epubMap[url] ?? null;
};

const getSetEPubLocation = (url: string) => (location: EpubView['location']) => {
  const epubMap = getEPubMap();

  try {
    epubMap[url] = location;
    localStorage.setItem(epubKey, JSON.stringify(epubMap));
  } catch (error) {
    console.error('Failed to save EPub location:', error);
  }
};

const useEPubLocation = (url: string): [EpubView['location'], (url: string) => void] => {
  const currentLocation = getLocation(url);
  const setEPubLocation = useCallback(getSetEPubLocation(url), []);

  return [currentLocation, setEPubLocation];
};

export default useEPubLocation;

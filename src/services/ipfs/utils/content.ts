import isSvg from 'is-svg';
import { PATTERN_HTTP, PATTERN_IPFS_HASH } from 'src/constants/patterns';
import { Option } from 'src/types';
import { shortenString } from 'src/utils/string';
import { toString as uint8ArrayToAsciiString } from 'uint8arrays/to-string';
import { IPFSContent, IPFSContentDetails, IpfsContentType, IpfsGatewayContentType } from '../types';
import { getResponseResult, onProgressCallback } from './stream';

function createObjectURL(rawData: Uint8Array, type: string) {
  const blob = new Blob([rawData], { type });
  return URL.createObjectURL(blob);
}

function createImgData(rawData: Uint8Array, type: string) {
  const imgBase64 = uint8ArrayToAsciiString(rawData, 'base64');
  const file = `data:${type};base64,${imgBase64}`;
  return file;
}

// eslint-disable-next-line import/no-unused-modules
export const detectGatewayContentType = (
  mime: string | undefined
): Option<IpfsGatewayContentType> => {
  if (mime) {
    if (mime.includes('video')) {
      return 'video';
    }

    if (mime.includes('audio')) {
      return 'audio';
    }

    if (mime.includes('epub')) {
      return 'epub';
    }
  }
  return undefined;
};

const basic = /\s?<!doctype html>|(<html\b[^>]*>|<body\b[^>]*>|<x-[^>]+>)+/i;

function isHtml(string: string) {
  const newString = string.trim().slice(0, 1000);
  return basic.test(newString);
}

// eslint-disable-next-line import/no-unused-modules
export const chunksToBlob = (chunks: Array<Uint8Array>, mime: string | undefined) =>
  new Blob(chunks, mime ? { type: mime } : {});

// eslint-disable-next-line import/no-unused-modules
export const mimeToBaseContentType = (mime: string | undefined): IpfsContentType => {
  if (!mime) {
    return 'other';
  }

  const initialType = detectGatewayContentType(mime);
  if (initialType) {
    return initialType;
  }

  if (mime.indexOf('text/plain') !== -1 || mime.indexOf('application/xml') !== -1) {
    return 'text';
  }
  if (mime.indexOf('image') !== -1) {
    return 'image';
  }
  if (mime.indexOf('application/pdf') !== -1) {
    return 'pdf';
  }
  return 'other';
};

// eslint-disable-next-line import/no-unused-modules, import/prefer-default-export
export const parseArrayLikeToDetails = async (
  content: Option<IPFSContent>,
  cid: string,
  onProgress?: onProgressCallback
): Promise<IPFSContentDetails> => {
  // try {
  if (!content || !content?.result) {
    return {
      gateway: true,
      text: cid.toString(),
      cid,
      type: content?.meta?.contentType,
    };
  }

  const { result, meta } = content;

  const { mime, contentType } = meta;

  if (!mime) {
    return {
      cid,
      gateway: true,
      text: `Can't detect MIME for ${cid.toString()}`,
    };
  }
  const contentCid = content.cid;

  const response: IPFSContentDetails = {
    link: `/ipfs/${cid}`,
    gateway: false,
    cid: contentCid,
    type: contentType,
  };

  if (detectGatewayContentType(mime)) {
    return { ...response, gateway: true };
  }

  const rawData = typeof result !== 'string' ? await getResponseResult(result, onProgress) : result;

  const isStringData = typeof rawData === 'string';

  // console.log(rawData);
  if (!rawData) {
    return {
      ...response,
      gateway: true,
      text: `Can't parse content for ${cid.toString()}`,
    };
  }

  // clarify text-content subtypes
  if (response.type === 'text') {
    // render svg as image
    if (!isStringData && isSvg(new Uint8Array(rawData))) {
      return {
        ...response,
        type: 'image',
        content: createImgData(rawData, 'image/svg+xml'),
      };
    }

    const str = isStringData ? rawData : uint8ArrayToAsciiString(rawData);

    if (str.match(PATTERN_IPFS_HASH)) {
      return {
        ...response,
        type: 'cid',
        content: str,
      };
    }
    if (str.match(PATTERN_HTTP)) {
      return {
        ...response,
        type: 'link',
        content: str,
      };
    }
    if (isHtml(str)) {
      return {
        ...response,
        type: 'html',
        gateway: true,
        content: cid.toString(),
      };
    }

    // TODO: search can bel longer for 42???!
    // also cover ipns links
    return {
      ...response,
      link: str.length > 42 ? `/ipfs/${cid}` : `/search/${str}`,
      type: 'text',
      text: shortenString(str),
      content: str,
    };
  }

  if (!isStringData) {
    if (response.type === 'image') {
      return { ...response, content: createImgData(rawData, mime) }; // file
    }
    if (response.type === 'pdf') {
      return {
        ...response,
        content: createObjectURL(rawData, mime),
        gateway: true,
      }; // file
    }
  }

  return response;
  // } catch (e) {
  //   console.log('----parseRawIpfsData', e, cid);
  //   return undefined;
  // }
};

export const contentToUint8Array = async (content: File | string): Promise<Uint8Array> => {
  return new Uint8Array(
    typeof content === 'string'
      ? new TextEncoder().encode(content)
      : new Uint8Array(await content.arrayBuffer())
  );
};

export const createTextPreview = (
  array: Uint8Array | undefined | string,
  contentType: IpfsContentType,
  previewLength = 150
) => {
  if (!array) {
    return undefined;
  }
  if (typeof array === 'string') {
    return array.slice(0, previewLength);
  }
  return contentType && contentType === 'text'
    ? uint8ArrayToAsciiString(array).slice(0, previewLength)
    : undefined;
};

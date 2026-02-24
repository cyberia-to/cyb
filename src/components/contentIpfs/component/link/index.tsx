import TextMarkdown from 'src/components/TextMarkdown';
import GatewayContent from '../gateway';

const EMBEDABLE_HOSTS = [
  'www.youtube.com',
  'youtube.com',
  'youtu.be',
  'www.twitter.com',
  'twitter.com',
  'www.reddit.com',
];

function LinkHttp({ url, preview }: { url: string; preview?: boolean }) {
  const urlObj = new URL(url);

  const canEmbed = EMBEDABLE_HOSTS.includes(urlObj.hostname);

  if (canEmbed) {
    return (
      <>
        {url}
        <GatewayContent url={url} />
      </>
    );
  }

  return <TextMarkdown preview={preview}>{url}</TextMarkdown>;
}

export default LinkHttp;

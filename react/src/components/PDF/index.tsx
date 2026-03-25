import Iframe from '../Iframe/Iframe';

type Props = {
  content: string;
  search?: boolean;
};

function Pdf({ content, search }: Props) {
  return (
    <Iframe
      height={search ? '400px' : '700px'}
      url={content}

      // TODO: USE loaded content
      // url={`${CYBER.CYBER_GATEWAY}${ipfsDataDetails?.link}`}
    />
  );
}

export default Pdf;

import { IPFSContent } from 'src/utils/ipfs/ipfs';
import styles from './Audio.module.scss';

function Audio({ content }: { content: IPFSContent }) {
  const { result, meta } = content;

  const audioBlob = new Blob([new Uint8Array(result)], { type: meta.mime });
  const audioUrl = URL.createObjectURL(audioBlob);

  return (
    <audio controls className={styles.wrapper}>
      <source src={audioUrl} type={meta.mime} />
      Your browser does not support the audio tag.
    </audio>
  );
}

export default Audio;

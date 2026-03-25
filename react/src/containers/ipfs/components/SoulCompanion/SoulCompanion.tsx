import { proxy } from 'comlink';
import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import Loader2 from 'src/components/ui/Loader2';
import { useScripting } from 'src/contexts/scripting/scripting';
import { IPFSContentDetails } from 'src/services/ipfs/types';
import type { ScriptMyCampanion } from 'src/services/scripting/types';
import { Option } from 'src/types';
import { ParticleCid } from 'src/types/base';
import { shortenString } from 'src/utils/string';
import styles from './SoulCompanion.module.scss';

type AskCompanionStatus = 'loading' | 'ready' | 'pending' | 'done' | 'error';

function SoulCompanion({
  cid,
  details,
  skip,
}: {
  cid: ParticleCid;
  details: Option<IPFSContentDetails>;
  skip: boolean;
}) {
  const [metaItems, setMetaItems] = useState<ScriptMyCampanion['metaItems']>([]);
  const [status, setStatus] = useState<AskCompanionStatus>('loading');
  const { rune, isSoulInitialized } = useScripting();

  useEffect(() => {
    if (!skip && isSoulInitialized && details) {
      if (details.type && details.type !== 'text' && details.text) {
        setStatus('done');
        setMetaItems([[{ type: 'text', text: `Skip companion for '${details.content}'.` }]]);
        return;
      }

      rune
        ?.askCompanion(
          cid,
          details!.type!,
          (details!.text! || '').substring(0, 255),
          proxy((_data = {}) => console.log('CALLBACK'))
        )
        .then((result) => {
          setMetaItems(result.metaItems);
          setStatus('done');
        });
    }
  }, [cid, skip, rune, isSoulInitialized, details]);

  useEffect(() => {
    setMetaItems([]);
  }, []);

  if (status !== 'done' && metaItems) {
    return <Loader2 text="" />;
  }

  console.log('metaItems', metaItems);

  return (
    <div className={styles.wrapper}>
      {metaItems.map((row, index) => (
        <ul className={styles.itemLinks} key={`soul_comp_row_${index}`}>
          {(Array.isArray(row) ? row : [row]).map((item, index) => (
            <li key={`soul_comp_col_${index}`}>
              {/* {item.type === 'text' && (
                <p className={styles.itemText}>{item.text}</p>
              )} */}
              {item.type === 'link' && (
                <Link className={styles.itemLink} to={item.url}>
                  {shortenString(item.title, 64)}
                </Link>
              )}
            </li>
          ))}
        </ul>
      ))}
    </div>
  );
}

export default SoulCompanion;

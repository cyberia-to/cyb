import cx from 'classnames';
import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Account } from 'src/components';
import ContentIpfs from 'src/components/contentIpfs/contentIpfs';
import HydrogenBalance from 'src/components/HydrogenBalance/HydrogenBalance';
import Karma from 'src/containers/application/Karma/Karma';
import AdviserMeta from 'src/containers/ipfs/components/AdviserMeta/AdviserMeta';
import { LLMAvatar } from 'src/containers/Search/LLMSpark/LLMSpark';
import AdviserHoverWrapper from 'src/features/adviser/AdviserHoverWrapper/AdviserHoverWrapper';
import useParticleDetails from 'src/features/particle/useParticleDetails';
import { isParticle as isParticleFunc } from 'src/features/particle/utils';
import { deleteLLMThread } from 'src/features/sense/redux/sense.redux';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { routes } from 'src/routes';
import styles from './SenseViewerHeader.module.scss';

function SenseViewerHeader({ selected }: { selected: string }) {
  const isParticle = isParticleFunc(selected || '');

  const { data: particleData } = useParticleDetails(selected!, {
    skip: !isParticle && !selected,
  });

  const [particleContentOpen, _setParticleContentOpen] = useState(false);

  const text = particleData?.text;
  const largeContent = text?.length > 200;

  if (isParticle) {
    return (
      <header className={styles.header}>
        {/* <ParticleAvatar particleId={selected} /> */}

        {/* {cutSenseItem(selected)} */}

        <div className={styles.meta}>
          <AdviserMeta
            cid={selected}
            type={particleData?.type}
            size={particleData?.content?.length}
          />
        </div>

        <Link
          className={cx(styles.title, {
            [styles.largeContent]: largeContent,
            [styles.fullContent]: largeContent && particleContentOpen,
          })}
          to={routes.oracle.ask.getLink(selected)}
        >
          <ContentIpfs
            // search
            cid={selected}
            details={particleData}
          />
        </Link>
      </header>
    );
  }

  return (
    <header className={styles.header_Neuron}>
      {/* need this wrapper to prevent jump */}
      <div className={styles.karma}>
        <Karma address={selected} />
      </div>

      {/* fix this check in parent */}
      {selected?.includes('bostrom') && <Account address={selected} avatar />}
      <Link to={routes.neuron.getLink(selected)}>
        <HydrogenBalance address={selected} />
      </Link>
    </header>
  );
}

export default SenseViewerHeader;

export function LLMHeader({ selectedId }: { selectedId?: string }) {
  const id = useAppSelector((state) => state.sense.llm.currentThreadId);
  const dispatch = useAppDispatch();

  return (
    <header
      className={styles.header}
      style={{
        flexDirection: 'row',
      }}
    >
      <LLMAvatar />

      <div
        style={{
          marginLeft: 'auto',
        }}
      >
        {id && (
          <AdviserHoverWrapper adviserContent="delete thread">
            <button
              type="button"
              onClick={() => {
                dispatch(deleteLLMThread({ id }));
              }}
            >
              ❌
            </button>
          </AdviserHoverWrapper>
        )}

        <Link
          style={{
            marginLeft: 10,
          }}
          to={`${routes.settings.path}/llm`}
        >
          ⚙️
        </Link>
      </div>
    </header>
  );
}

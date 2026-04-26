/**
 * particle-page — prysm spec implementation
 *
 * glass [fill × fill, depth background, overflow scroll]
 *   stack vertical [gap 2g, padding 3g]
 *     display [fill × auto, particle content]
 *     glass [fill × fix(5g), depth midground] — filter bar
 *     stack vertical [gap 0] — linked particles list
 */

import { useMemo } from 'react';
import ContentIpfs from 'src/components/contentIpfs/contentIpfs';
import type { IPFSContentDetails } from 'src/services/ipfs/types';
import type { IPFSContent } from 'src/services/ipfs/types';
import styles from './ParticlePage.module.scss';
import p from './prysm.module.scss';

type LinkedParticle = {
  cid: string;
  name: string;
  rank: number;
  maxRank: number;
};

type FilterBarProps = {
  outgoing: number;
  incoming: number;
  total: number;
  activeFilter: 'all' | 'outgoing' | 'incoming';
  onFilterChange: (filter: 'all' | 'outgoing' | 'incoming') => void;
};

function FilterBar({ outgoing, incoming, total, activeFilter, onFilterChange }: FilterBarProps) {
  return (
    <div className={`${p.glass} ${p['glass-midground']} ${styles.filterBar}`}>
      <div className={`${p['stack-h']} ${p['gap-2']}`}>
        {/* type filters */}
        <span
          className={`${p.text} ${p['text-body']} ${styles.filterItem} ${activeFilter === 'all' ? styles.filterActive : ''}`}
          onClick={() => onFilterChange('all')}
        >
          all
        </span>

        <div className={`${p.saber} ${p['saber-v']}`} />

        {/* direction counters */}
        <span
          className={`${p.text} ${p['text-body']} ${p.counter} ${styles.filterItem} ${activeFilter === 'outgoing' ? styles.filterActive : ''}`}
          onClick={() => onFilterChange('outgoing')}
        >
          {outgoing} →
        </span>

        <span className={`${p.text} ${p['text-caption']}`}>
          ⟡
        </span>

        <span
          className={`${p.text} ${p['text-body']} ${p.counter} ${styles.filterItem} ${activeFilter === 'incoming' ? styles.filterActive : ''}`}
          onClick={() => onFilterChange('incoming')}
        >
          → {incoming}
        </span>

        <div className={`${p.saber} ${p['saber-v']}`} />

        {/* total */}
        <span className={`${p.text} ${p['text-body']} ${p.counter}`}>
          {total}
        </span>
        <span className={`${p.text} ${p['text-caption']}`}>
          particles
        </span>
      </div>
    </div>
  );
}

function LinkedParticleRow({ name, rank, maxRank }: LinkedParticle) {
  const fillPercent = maxRank > 0 ? (rank / maxRank) * 100 : 0;

  return (
    <>
      <div className={`${p.glass} ${p['glass-midground']} ${styles.particleRow}`}>
        <div className={`${p['stack-h']} ${p['gap-1']}`} style={{ width: '100%' }}>
          <span className={`${p.text} ${p['text-body']} ${p.fill}`}>
            {name}
          </span>
          <div className={`${p.pill} ${p['pill-progress']} ${styles.rankPill}`}>
            <div
              className={p['pill-progress-fill']}
              style={{
                width: `${fillPercent}%`,
                backgroundColor: 'var(--emotion-joy)',
              }}
            />
          </div>
        </div>
      </div>
      <div className={`${p.saber} ${p['saber-h']}`} />
    </>
  );
}

type ParticlePageProps = {
  cid: string;
  details?: IPFSContentDetails;
  content?: IPFSContent;
  linkedParticles?: LinkedParticle[];
  outgoing?: number;
  incoming?: number;
};

function ParticlePage({
  cid,
  details,
  content,
  linkedParticles = [],
  outgoing = 0,
  incoming = 0,
}: ParticlePageProps) {
  const total = outgoing + incoming;
  const maxRank = useMemo(
    () => Math.max(...linkedParticles.map((lp) => lp.rank), 1),
    [linkedParticles]
  );

  return (
    <div className={`${p.glass} ${p['glass-background']} ${p.scroll} ${styles.page}`}>
      <div className={`${p['stack-v']} ${p['gap-2']} ${p['pad-3']}`}>
        {/* content render */}
        {details && (
          <div className={styles.display}>
            <ContentIpfs content={content} details={details} cid={cid} />
          </div>
        )}

        {/* cyberlink filter bar */}
        <FilterBar
          outgoing={outgoing}
          incoming={incoming}
          total={total}
          activeFilter="all"
          onFilterChange={() => {}}
        />

        {/* linked particles list */}
        <div className={`${p['stack-v']} ${p['gap-0']}`}>
          {linkedParticles.map((lp) => (
            <LinkedParticleRow key={lp.cid} {...lp} maxRank={maxRank} />
          ))}
        </div>
      </div>
    </div>
  );
}

export default ParticlePage;

import { Tabs } from 'src/components';
import { Route, Routes, useParams } from 'react-router-dom';
import { useMemo } from 'react';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import CyberlinksGraphContainer from 'src/features/cyberlinks/CyberlinksGraph/CyberlinksGraphContainer';
import { useRobotContext } from '../robot.context';
import TreedView from './ui/TreedView';
import styles from './Brain.module.scss';
import GraphView from './ui/GraphView';
import useGraphLimit from './useGraphLimit';
import Particles from './ui/Particles/Particles';

enum TabsKey {
  graph3d = 'graph3d',
  graph = 'graph',
  list = 'list',
}

function Brain() {
  const { address } = useRobotContext();

  const params = useParams();

  const selected = params['*'] || TabsKey.graph;

  useAdviserTexts({
    defaultText: useMemo(
      () => (
        <>
          neuron public knowledge cybergraph <br />
          {/* {selected === TabsKey.graph3d && (
            <> that is how last {LIMIT_GRAPH} cyberlinks looks like </>
          )} */}
        </>
      ),
      []
    ),
  });

  return (
    <div className={styles.wrapper}>
      <div className={styles.tabs}>
        <Tabs
          options={[
            {
              key: TabsKey.graph3d,
              to: './graph3d',
              text: '3d graph',
            },
            {
              key: TabsKey.graph,
              to: './graph',
              text: '2d graph',
            },
            {
              key: TabsKey.list,
              to: './list',
              text: 'last cyberlinks',
            },
            {
              key: TabsKey.list,
              to: './particles',
              text: 'particles',
            },
          ]}
          selected={selected}
        />
      </div>

      <Routes>
        {['/', 'graph'].map((path) => (
          <Route
            key={path}
            path={path}
            element={<Graph2d address={address} />}
          />
        ))}

        <Route path="list" element={<TreedView address={address} />} />
        <Route path="particles" element={<Particles />} />

        <Route path="graph3d" element={<GraphView address={address} />} />
      </Routes>
    </div>
  );
}

export default Brain;

function Graph2d({ address }) {
  const { limit } = useGraphLimit();

  return <CyberlinksGraphContainer toPortal limit={limit} address={address} />;
}

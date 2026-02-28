import { useCallback, useEffect, useRef, useState } from 'react';
import ForceGraph3D from 'react-force-graph-3d';
import GraphActionBar from '../graph/GraphActionBar/GraphActionBar';

import styles from './CyberlinksGraph.module.scss';
import GraphHoverInfo from './GraphHoverInfo/GraphHoverInfo';

type Props = {
  data: any;
  // currentAddress?: string;
  size?: number;
};

// before zoom in
const INITIAL_CAMERA_DISTANCE = 2500;
const DEFAULT_CAMERA_DISTANCE = 1300;
const CAMERA_ZOOM_IN_EFFECT_DURATION = 5000;
const CAMERA_ZOOM_IN_EFFECT_DELAY = 500;

// D3-timer creates setInterval(poke, 1000) on every animation frame,
// and WebKit accumulates them causing 85% CPU.
// This hook intercepts setInterval before ForceGraph3D mounts,
// tracks all IDs, and periodically sweeps them to prevent accumulation.
function useIntervalSweep() {
  const trackedIds = useRef<ReturnType<typeof setInterval>[]>([]);
  const origRef = useRef<typeof window.setInterval | null>(null);

  useEffect(() => {
    const real = window.setInterval.bind(window);
    origRef.current = window.setInterval;

    (window as any).setInterval = (...args: any[]) => {
      const id = real(...args);
      trackedIds.current.push(id);
      return id;
    };

    // Aggressively sweep accumulated D3 timer intervals every 100ms.
    // D3-timer sleep() creates a new setInterval(poke, 1000) on every animation frame.
    // WebKit accumulates hundreds of these, causing 85% CPU.
    // Keep only the 2 most recent (the app needs some legitimate intervals).
    const sweepInterval = real(() => {
      if (trackedIds.current.length > 5) {
        const toSweep = trackedIds.current.splice(0, trackedIds.current.length - 2);
        toSweep.forEach((id) => clearInterval(id));
      }
    }, 100);

    return () => {
      clearInterval(sweepInterval);
      if (origRef.current) {
        window.setInterval = origRef.current;
        origRef.current = null;
      }
      trackedIds.current.forEach((id) => clearInterval(id));
      trackedIds.current = [];
    };
  }, []);
}

function CyberlinksGraph({ data, size, minVersion }: Props) {
  const [isRendering, setRendering] = useState(true);
  const [touched, setTouched] = useState(false);
  const [hoverNode, setHoverNode] = useState(null);

  const fgRef = useRef<any>();

  // Prevent D3 timer setInterval cascade from consuming 85% CPU
  useIntervalSweep();

  // debug, remove later
  useEffect(() => {
    if (isRendering) {
      console.time('rendering');
    } else {
      console.timeEnd('rendering');
    }
  }, [isRendering]);

  // initial camera position, didn't find via props
  useEffect(() => {
    if (!fgRef.current) {
      return;
    }
    fgRef.current.cameraPosition({ z: INITIAL_CAMERA_DISTANCE });
  }, []);

  // initial loading camera zoom effect
  useEffect(() => {
    if (!fgRef.current || isRendering) {
      return;
    }

    setTimeout(() => {
      if (!fgRef.current) {
        return;
      }

      fgRef.current.cameraPosition(
        { z: DEFAULT_CAMERA_DISTANCE },
        null,
        CAMERA_ZOOM_IN_EFFECT_DURATION
      );
    }, CAMERA_ZOOM_IN_EFFECT_DELAY);
  }, [isRendering]);

  useEffect(() => {
    if (!fgRef.current) {
      return;
    }

    function onTouch() {
      setTouched(true);
    }

    fgRef.current.controls().addEventListener('start', onTouch);

    return () => {
      if (fgRef.current) {
        fgRef.current.controls().removeEventListener('start', onTouch);
      }
    };
  }, []);

  // orbit camera using requestAnimationFrame (avoids 10ms setInterval CPU drain)
  useEffect(() => {
    if (!fgRef.current || touched || isRendering) {
      return;
    }

    let angle = 0;
    let rafId: number | null = null;
    let lastTime = 0;

    const orbit = (time: number) => {
      if (time - lastTime >= 33) { // ~30fps
        lastTime = time;
        if (fgRef.current) {
          fgRef.current.cameraPosition({
            x: DEFAULT_CAMERA_DISTANCE * Math.sin(angle),
            z: DEFAULT_CAMERA_DISTANCE * Math.cos(angle),
          });
          angle += Math.PI / 900; // same visual speed at 30fps
        }
      }
      rafId = requestAnimationFrame(orbit);
    };

    const timeout = setTimeout(() => {
      rafId = requestAnimationFrame(orbit);
    }, CAMERA_ZOOM_IN_EFFECT_DURATION + CAMERA_ZOOM_IN_EFFECT_DELAY);

    return () => {
      clearTimeout(timeout);
      if (rafId !== null) cancelAnimationFrame(rafId);
    };
  }, [touched, isRendering]);

  const handleNodeClick = useCallback((node) => {
    if (!fgRef.current) {
      return;
    }

    const distance = 300;
    const distRatio = 1 + distance / Math.hypot(node.x, node.y, node.z);

    fgRef.current.cameraPosition(
      { x: node.x * distRatio, y: node.y * distRatio, z: node.z * distRatio },
      node,
      5000
    );
  }, []);

  const handleLinkClick = useCallback((link) => {
    if (!fgRef.current) {
      return;
    }

    const node = link.target;
    const distance = 300;
    const distRatio = 1 + distance / Math.hypot(node.x, node.y, node.z);

    fgRef.current.cameraPosition(
      { x: node.x * distRatio, y: node.y * distRatio, z: node.z * distRatio },
      node,
      5000
    );
  }, []);

  const handleNodeRightClick = useCallback((node) => {
    window.open(`${window.location.origin}/ipfs/${node.id}`, '_blank');
  }, []);

  const handleLinkRightClick = useCallback((link) => {
    window.open(`${window.location.origin}/network/bostrom/tx/${link.name}`, '_blank');
  }, []);

  const handleEngineStop = useCallback(() => {
    console.log('ForceGraph3D engine stopped!');
    setRendering(false);
  }, []);

  return (
    <div
      style={{
        minHeight: size,
        position: 'relative',
      }}
    >
      {isRendering && <div className={styles.loaderWrapper}>rendering data...</div>}

      <ForceGraph3D
        height={size}
        width={size}
        ref={fgRef}
        graphData={data}
        showNavInfo={false}
        backgroundColor="rgba(0, 0, 0, 0)"
        warmupTicks={420}
        cooldownTicks={0}
        enableNodeDrag={false}
        enablePointerInteraction
        enableNavigationControls
        // nodeLabel="id"
        nodeColor={() => 'rgba(0,100,235,1)'}
        nodeOpacity={1.0}
        nodeRelSize={8}
        onNodeHover={setHoverNode}
        linkColor={
          // not working
          (_link) =>
            // link.subject && link.subject === currentAddress
            //   ? 'red'
            'rgba(9,255,13,1)'
        }
        linkLabel=""
        linkWidth={4}
        linkCurvature={0.2}
        linkOpacity={0.7}
        linkDirectionalParticles={1}
        linkDirectionalParticleColor={() => 'rgba(9,255,13,1)'}
        linkDirectionalParticleWidth={4}
        linkDirectionalParticleSpeed={0.015}
        // linkDirectionalArrowRelPos={1}
        // linkDirectionalArrowLength={10}
        // linkDirectionalArrowColor={() => 'rgba(9,255,13,1)'}

        onNodeClick={!minVersion ? handleNodeRightClick : undefined}
        onNodeRightClick={handleNodeClick}
        onLinkClick={handleLinkRightClick}
        onLinkRightClick={handleLinkClick}
        onEngineStop={handleEngineStop}
      />

      {!minVersion && (
        <>
          <GraphHoverInfo
            node={hoverNode}
            camera={fgRef.current?.camera()}
            size={size || window.innerWidth}
          />
          <GraphActionBar />
        </>
      )}
    </div>
  );
}

export default CyberlinksGraph;

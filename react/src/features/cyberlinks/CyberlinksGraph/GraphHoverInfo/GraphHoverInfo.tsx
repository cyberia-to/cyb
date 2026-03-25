import { useMemo } from 'react';
import ContentItem from 'src/components/ContentItem/contentItem';
import * as THREE from 'three';
import styles from './GraphHoverInfo.module.scss';

// fix
type Props = {
  node: any;
  camera: any;
  size: number;
};

function HoverInfo({ node, camera, size, left, top, right }: Props) {
  let l = left;
  let t = top;

  const calc = useMemo(() => {
    if (!node || !camera) {
      return null;
    }

    const { x, y, z } = node;
    const vector = new THREE.Vector3(x, y, z);
    vector.project(camera);

    const widthHalf = window.innerWidth / 2;
    const heightHalf = window.innerHeight / 2;

    const posX = vector.x * widthHalf + widthHalf;
    const posY = -(vector.y * heightHalf) + heightHalf;

    return {
      posX,
      posY,
    };
  }, [camera, node]);

  if (calc) {
    const { posX, posY } = calc;
    l = posX;
    t = posY;
  }

  if (!node?.id) {
    return null;
  }

  const isCid = node.id.startsWith('Qm');

  if (!isCid) {
  }

  return (
    <div
      className={styles.hoverInfo}
      style={{
        top: t,
        left: l,
        right,
      }}
    >
      {/* {node.id} */}
      <ContentItem cid={node.id} />
    </div>
  );
}

export default HoverInfo;

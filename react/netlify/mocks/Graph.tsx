import React from 'react';

console.log(React.version);

export function GraphMock() {
  return (
    <div
      style={{
        position: 'absolute',
        zIndex: 10,
        color: 'white',
        top: '50%',
      }}
    >
      graph is disabled for netlify builds
    </div>
  );
}

const Graph = GraphMock;

// eslint-disable-next-line import/no-unused-modules
export default Graph;

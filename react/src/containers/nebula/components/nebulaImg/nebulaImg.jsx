function NebulaImg() {
  return (
    <svg viewBox="0 0 350 350" version="1.1" xmlns="http://www.w3.org/2000/svg">
      <radialGradient id="a">
        {/* <stop offset=".1" stopColor="#000" /> */}
        <stop stopColor="#aa00ff">
          <animate
            attributeName="stop-color"
            values="#aa00ff; #258; #aa00ff"
            dur="5s"
            repeatCount="indefinite"
          />
        </stop>
        <stop offset=".2" stopColor="#00b8d4">
          <animate attributeName="offset" values=".2; .15; .2" dur="5s" repeatCount="indefinite" />
          <animate
            attributeName="stop-color"
            values="#00b8d4; #00e5ff; #00b8d4"
            dur="5s"
            repeatCount="indefinite"
          />
        </stop>
        <stop offset=".35" stopColor="#dd2c00">
          <animate
            attributeName="offset"
            values=".35; .37; .35"
            dur="5s"
            repeatCount="indefinite"
          />
          <animate
            attributeName="stop-color"
            values="#ff3d00; #dd2c00; #ff3d00"
            dur="5s"
            repeatCount="indefinite"
          />
        </stop>

        <stop offset=".6" stopColor="transparent">
          <animate attributeName="offset" values=".6; .7; .6" dur="5s" repeatCount="indefinite" />
        </stop>
      </radialGradient>
      <filter id="b">
        <feTurbulence baseFrequency=".2" />
        <feTurbulence type="fractalNoise" baseFrequency=".01" numOctaves="5" />
        <feDisplacementMap in="SourceGraphic" scale="99" />
      </filter>
      <circle cx="50%" cy="50%" r="71%" fill="url(#a)" filter="url(#b)" />
    </svg>
  );
}

export default NebulaImg;

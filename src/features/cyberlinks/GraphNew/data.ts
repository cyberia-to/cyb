export type Node = {
  [key: string]: unknown;
  id: string;
  value?: number;
  color?: string;
  size?: number;
};

export type Link = {
  source: string;
  target: string;
  time?: string;
  width?: number;
  color?: string;
};

const _randomIntFromInterval = (min: number, max: number): number => {
  return Math.floor(Math.random() * (max - min + 1) + min);
};

const _colors = ['#88C6FF', '#FF99D2', '#2748A4'];

// export const links: Link[] = silkRoadCase.map((d) => ({
//   source: d.source,
//   target: d.target,
//   color: colors[Math.floor(Math.random() * colors.length)],
//   width: Math.random() * 2,
//   date: new Date(d.time),
// }));

// export const nodes: Node[] = Array.from(
//   new Set([
//     ...silkRoadCase.map((d) => d.source),
//     ...silkRoadCase.map((d) => d.target),
//   ])
// ).map((id, i) => ({
//   id,
//   value: i % randomIntFromInterval(0, 10000),
//   size: Math.floor(Math.random() * 10) + 1,
// }));

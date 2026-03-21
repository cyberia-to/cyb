import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execSync } from 'node:child_process';
import { TSBuilder } from '@cosmwasm/ts-codegen';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const cybTsRoot = path.resolve(__dirname, '..');
const cwCyberRoot = path.resolve(cybTsRoot, '../cw-cyber');

execSync('./scripts/generate-lithium-schema.sh', {
  cwd: cwCyberRoot,
  stdio: 'inherit',
});

const contracts = [
  { name: 'LitiumCore', dir: path.join(cwCyberRoot, 'contracts/litium-core/schema') },
  { name: 'LitiumStake', dir: path.join(cwCyberRoot, 'contracts/litium-stake/schema') },
  { name: 'LitiumWrap', dir: path.join(cwCyberRoot, 'contracts/litium-wrap/schema') },
  { name: 'LitiumRefer', dir: path.join(cwCyberRoot, 'contracts/litium-refer/schema') },
  { name: 'LitiumMine', dir: path.join(cwCyberRoot, 'contracts/litium-mine/schema') },
];

const outPath = path.join(cybTsRoot, 'src/generated/lithium');
const builder = new TSBuilder({
  contracts,
  outPath,
});

await builder.build();
console.log(`[ts-codegen] generated lithium clients at ${outPath}`);

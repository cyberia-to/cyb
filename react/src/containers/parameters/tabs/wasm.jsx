import { Pane } from '@cybercongress/gravity';
import { CardStatisics } from '../../../components';

function WasmParam({ data }) {
  return (
    <Pane display="grid" gridTemplateColumns="repeat(auto-fit, minmax(250px, 1fr))" gridGap="20px">
      <CardStatisics title="code upload access" value="Everybody" />
      <CardStatisics title="instantiate default permission" value="Everybody" />
      <CardStatisics title="max wasm code size" value="1200 kB" />
    </Pane>
  );
}

export default WasmParam;

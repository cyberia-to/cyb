import { Tab, Tablist } from '@cybercongress/gravity';
import { useState } from 'react';
import RenderAbiExecute from './RenderAbiExecute';
import RenderAbiQuery from './RenderAbiQuery';
import SelectFile from './SelectFile';
import useParseJsonSchema from './useParseJsonSchema';

// import testJson from '../query_msg.json';
// import testJsonTx from '../cw20_execute_msg.json';

function Btn({ onSelect, checkedSwitch, text, ...props }) {
  return (
    <Tab
      isSelected={checkedSwitch}
      onSelect={onSelect}
      color="#36d6ae"
      boxShadow="0px 0px 10px #36d6ae"
      minWidth="100px"
      marginX={0}
      paddingX={10}
      paddingY={10}
      fontSize="18px"
      height={42}
      {...props}
    >
      {text}
    </Tab>
  );
}

function RenderAbi({ contractAddress, updateFnc }) {
  const [selected, setSelected] = useState('query');
  const [fileAbiQuery, setFileAbiQuery] = useState(null);
  const [fileAbiExecute, setFileAbiExecute] = useState(null);
  const { dataObj: schemaQuery } = useParseJsonSchema(fileAbiQuery);
  const { dataObj: schemaExecute } = useParseJsonSchema(fileAbiExecute);

  let content;

  if (selected === 'query') {
    if (fileAbiQuery === null) {
      content = <SelectFile stateCallback={setFileAbiQuery} />;
    } else {
      content = <RenderAbiQuery schema={schemaQuery} contractAddress={contractAddress} />;
    }
  }

  if (selected === 'execute') {
    if (fileAbiExecute === null) {
      content = <SelectFile text="Upload execute schema" stateCallback={setFileAbiExecute} />;
    } else {
      content = (
        <RenderAbiExecute
          schema={schemaExecute}
          contractAddress={contractAddress}
          updateFnc={updateFnc}
        />
      );
    }
  }

  return (
    <div>
      <Tablist
        display="grid"
        gridTemplateColumns="150px 150px"
        gridGap="8px"
        justifyContent="flex-start"
        marginBottom={30}
      >
        <Btn
          text="Query"
          checkedSwitch={selected === 'query'}
          onSelect={() => setSelected('query')}
        />
        <Btn
          text="Execute"
          checkedSwitch={selected === 'execute'}
          onSelect={() => setSelected('execute')}
        />
      </Tablist>

      {content}
    </div>
  );
}

export default RenderAbi;

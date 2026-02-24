/* eslint-disable jsx-a11y/control-has-associated-label */
import { useState } from 'react';
import { useQueryClient } from 'src/contexts/queryClient';
import JsonSchemaParse from './JsonSchemaParse';

function RenderAbiQuery({ contractAddress, schema }) {
  const queryClient = useQueryClient();
  const [contractResponse, setContractResponse] = useState(null);

  const runQuery = async ({ formData }, key) => {
    if (!queryClient || !formData) {
      return;
    }
    setContractResponse(null);
    try {
      const queryResponseResult = await queryClient.queryContractSmart(contractAddress, formData);
      console.log(`queryResponseResult`, queryResponseResult);
      setContractResponse({ result: queryResponseResult, key });
    } catch (e) {
      console.log(`error`, e);
      // setQueryResponse({ error: `Query error: ${e.message}` });
    }
  };

  let itemAutoForm = [];

  if (schema.length > 0) {
    itemAutoForm = schema.map((items, key) => {
      // const bridge = makeBridge(items);
      // const key = uuidv4();
      return (
        <JsonSchemaParse
          key={key}
          schema={items}
          contractResponse={contractResponse}
          keyItem={key}
          onSubmitFnc={runQuery}
        />
      );
    });
  }

  return itemAutoForm.length > 0 && itemAutoForm;
}

export default RenderAbiQuery;

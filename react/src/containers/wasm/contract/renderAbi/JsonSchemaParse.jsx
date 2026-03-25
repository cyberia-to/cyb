import JsonSchema from '@rjsf/core';
import { Dots } from '../../../../components';
import { JsonView } from '../../ui/ui';

const styles = {
  padding: '15px',
  display: 'flex',
  flexDirection: 'column',
  gridGap: '20px',
  borderBottom: ' 1px solid #38d6aea8',
  paddingBottom: '37px',
};

function CustomDescriptionField({ id, description }) {
  return (
    <div id={id} style={{ display: 'none' }}>
      {description}
    </div>
  );
}

const fields = {
  DescriptionField: CustomDescriptionField,
};

function JsonSchemaParse({
  schema,
  executing,
  activeKey,
  contractResponse,
  keyItem,
  onSubmitFnc,
  disabledOnSubmit,
}) {
  return (
    <div style={styles}>
      <JsonSchema
        key={keyItem}
        schema={schema}
        fields={fields}
        autoComplete="off"
        // uiSchema={uiSchema}
        onSubmit={(e) => onSubmitFnc(e, keyItem)}
        // onSubmit={(e) => runExecute(e)}
      >
        <div>
          <button disabled={disabledOnSubmit} className="btn" type="submit">
            {activeKey === keyItem && executing ? (
              <p>
                Executing
                <Dots />
              </p>
            ) : (
              'Submit'
            )}
          </button>
        </div>
      </JsonSchema>
      {contractResponse !== null && contractResponse.key === keyItem && (
        <div>
          <span>Response:</span>
          <JsonView
            src={
              typeof contractResponse.result === 'object' && contractResponse.result !== null
                ? contractResponse.result
                : {
                    result: contractResponse.result,
                  }
            }
          />
        </div>
      )}
    </div>
  );
}

export default JsonSchemaParse;

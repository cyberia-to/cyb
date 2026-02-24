import { useEffect, useMemo, useState } from 'react';
import Display from 'src/components/containerGradient/Display/Display';
import DisplayTitle from 'src/components/containerGradient/DisplayTitle/DisplayTitle';
import withRouter from 'src/components/helpers/withRouter';
import Table from 'src/components/Table/Table';
import StatusTxs from 'src/components/TableTxsInfinite/component/StatusTxs';
import TxHash from 'src/components/TableTxsInfinite/component/txHash';
import { useBlockByHeightQuery } from 'src/generated/graphql';
import { MainContainer, TextTable } from '../../components';
import ActionBarContainer from '../Search/ActionBarContainer';
import InformationBlock from './informationBlock';

const initialState = {
  height: null,
  timestamp: null,
  hash: null,
  transactions: [],
};

enum ColumnsTable {
  status = 'status',
  tx = 'tx',
  messages = 'messages',
}

function BlockDetails({ router }) {
  const { idBlock } = router.params;
  const [blockInfo, setBlockInfo] = useState(initialState);
  const { data } = useBlockByHeightQuery({
    variables: {
      blockId: idBlock,
    },
  });

  useEffect(() => {
    if (data?.block && Object.keys(data.block).length > 0) {
      setBlockInfo(data.block[0]);
    } else {
      setBlockInfo(initialState);
    }
    return () => setBlockInfo(initialState);
  }, [data]);

  const tableData = useMemo(() => {
    return data?.block[0].transactions.map((item) => {
      return {
        [ColumnsTable.status]: (
          <TextTable>
            <StatusTxs success={item.success} />
          </TextTable>
        ),
        [ColumnsTable.messages]: <TextTable>{item.messages.length || 0}</TextTable>,

        [ColumnsTable.tx]: (
          <TextTable>
            <TxHash hash={item.hash} />
          </TextTable>
        ),
      };
    });
  }, [data]);

  return (
    <div>
      <MainContainer>
        <InformationBlock numbTx={blockInfo.transactions} data={blockInfo} />
        <Display color="blue" title={<DisplayTitle title="Transactions" />}>
          <Table
            data={tableData || []}
            columns={Object.values(ColumnsTable).map((item) => ({
              header: item,
              accessorKey: item,
              cell: (info) => info.getValue(),
            }))}
          />
        </Display>
      </MainContainer>
      <ActionBarContainer valueSearchInput={idBlock} keywordHash={idBlock} />
    </div>
  );
}

export default withRouter(BlockDetails);

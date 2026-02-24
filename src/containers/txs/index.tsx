import { Pane, TableEv as Table } from '@cybercongress/gravity';
import { Link } from 'react-router-dom';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import { useTransactionsSubscription } from 'src/generated/graphql';
import { v4 as uuidv4 } from 'uuid';
import { Loading, MainContainer, MsgType, TextTable } from '../../components';
import statusTrueImg from '../../image/ionicons_svg_ios-checkmark-circle.svg';
import statusFalseImg from '../../image/ionicons_svg_ios-close-circle.svg';
import { formatNumber, trimString } from '../../utils/utils';

function Txs() {
  const { loading, error, data } = useTransactionsSubscription();

  useAdviserTexts({
    defaultText: 'Transactions',
  });

  function renderRows() {
    return (data?.transaction || []).map((item, index) => (
      <Table.Row
        paddingX={0}
        paddingY={5}
        borderTop={index === 0 ? 'none' : '1px solid #3ab79340'}
        borderBottom="none"
        display="flex"
        minHeight="48px"
        height="fit-content"
        key={item.hash}
      >
        <Table.TextCell flex={0.5} textAlign="center">
          <TextTable>
            <img
              style={{ width: '20px', height: '20px', marginRight: '5px' }}
              src={item.success ? statusTrueImg : statusFalseImg}
              alt="statusImg"
            />
          </TextTable>
        </Table.TextCell>
        <Table.TextCell textAlign="center">
          <TextTable>
            <Link to={`/network/bostrom/tx/${item.hash}`}>{trimString(item.hash, 6, 6)}</Link>
          </TextTable>
        </Table.TextCell>
        <Table.TextCell flex={1.3} textAlign="center">
          {/* <TextTable>{dateFormat(item.height, 'dd/mm/yyyy, HH:MM:ss')}</TextTable> */}
          <TextTable>{formatNumber(item.height)}</TextTable>
        </Table.TextCell>
        <Table.TextCell textAlign="start">
          <TextTable display="flex" alignItems="start" flexDirection="column">
            {item.messages.length > 4 ? (
              <Pane display="flex" alignItems="center">
                <MsgType key={uuidv4()} type={item.messages[0]['@type']} />
                <div style={{ marginLeft: '5px' }}>({item.messages.length} messages)</div>
              </Pane>
            ) : (
              item.messages.map((items, _i) => {
                const key = uuidv4();
                return <MsgType key={key} type={items['@type']} />;
              })
            )}
          </TextTable>
        </Table.TextCell>
      </Table.Row>
    ));
  }

  return (
    <MainContainer>
      <Table>
        <Table.Head
          style={{
            backgroundColor: '#000',
            borderBottom: '1px solid #ffffff80',
            marginTop: '10px',
            padding: 7,
            paddingBottom: '10px',
          }}
        >
          <Table.TextHeaderCell flex={0.5} textAlign="center">
            <TextTable>status</TextTable>
          </Table.TextHeaderCell>
          <Table.TextHeaderCell textAlign="center">
            <TextTable>tx</TextTable>
          </Table.TextHeaderCell>
          <Table.TextHeaderCell flex={1.3} textAlign="center">
            <TextTable>block height</TextTable>
          </Table.TextHeaderCell>
          <Table.TextHeaderCell textAlign="center">
            <TextTable>type</TextTable>
          </Table.TextHeaderCell>
        </Table.Head>

        {loading ? (
          <Loading />
        ) : data?.transaction.length ? (
          <Table.Body
            style={{
              backgroundColor: '#000',
              overflowY: 'hidden',
              padding: 7,
            }}
          >
            {renderRows()}
          </Table.Body>
        ) : error ? (
          <p>{error.message}</p>
        ) : (
          'No data'
        )}
      </Table>
    </MainContainer>
  );
}

export default Txs;

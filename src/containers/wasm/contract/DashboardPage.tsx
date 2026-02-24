import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { CardStatisics, Dots, MainContainer, NoItems } from 'src/components';
import Loader2 from 'src/components/ui/Loader2';
import { BASE_DENOM } from 'src/constants/config';
import { useQueryClient } from 'src/contexts/queryClient';
import { useWasmDashboardPageQuery } from 'src/generated/graphql';
import { formatNumber } from '../../../utils/utils';
import { ContainerCardStatisics, ContainerCol } from '../ui/ui';
import ContractTable from './ContractTable';

const PAGE_SIZE = 50;

const useGetContracts = (offset) => {
  const [dataContracts, setDataContracts] = useState([]);
  const [dataAggregate, setDataAggregate] = useState(null);
  const { loading, data } = useWasmDashboardPageQuery({
    variables: {
      offset: offset * PAGE_SIZE,
      limit: PAGE_SIZE,
    },
  });

  // effect  not needed
  useEffect(() => {
    if (data?.contracts && data.contracts_aggregate) {
      setDataContracts((items) => [...items, ...data.contracts]);
      if (data.contracts_aggregate.aggregate) {
        setDataAggregate(data.contracts_aggregate.aggregate);
      }
    }
  }, [data]);

  return { dataContracts, dataAggregate, loading };
};

const useGetCodes = () => {
  const queryClient = useQueryClient();
  const [codes, setCodes] = useState(null);

  useEffect(() => {
    const getCodes = async () => {
      try {
        if (queryClient) {
          const resposeCodes = await queryClient.getCodes();

          if (resposeCodes && resposeCodes.length > 0) {
            setCodes(resposeCodes.length);
          } else {
            setCodes(0);
          }
        }
      } catch (error) {
        console.log(`error getCodes`, error);
        setCodes(0);
      }
    };
    getCodes();
  }, [queryClient]);

  return { codes };
};

function DashboardPage() {
  const [offset, setOffset] = useState(0);
  const { dataContracts, dataAggregate, loading } = useGetContracts(offset);
  const { codes } = useGetCodes();

  return (
    <MainContainer>
      <ContainerCol>
        <ContainerCardStatisics>
          <CardStatisics
            title="Contracts"
            value={dataAggregate !== null ? formatNumber(dataAggregate.count) : <Dots />}
          />
          <CardStatisics
            title={`Income, ${BASE_DENOM.toUpperCase()}`}
            value={
              dataAggregate !== null && dataAggregate.sum.fees !== null ? (
                formatNumber(dataAggregate.sum.fees)
              ) : (
                <Dots />
              )
            }
          />
          <CardStatisics
            title="Gas used"
            value={
              dataAggregate !== null && dataAggregate.sum.gas !== null ? (
                formatNumber(dataAggregate.sum.gas)
              ) : (
                <Dots />
              )
            }
          />
          <CardStatisics
            title="Total txs"
            value={
              dataAggregate !== null && dataAggregate.sum.tx !== null ? (
                formatNumber(dataAggregate.sum.tx)
              ) : (
                <Dots />
              )
            }
          />
          <Link to="/libs">
            <CardStatisics
              title="Libs"
              value={codes !== null ? formatNumber(codes) : <Dots />}
              link
            />
          </Link>
        </ContainerCardStatisics>

        {loading ? (
          <Loader2 />
        ) : dataContracts.length === 0 ? (
          <NoItems text="No contracts" />
        ) : (
          <ContractTable
            contracts={dataContracts}
            setOffset={setOffset}
            count={dataAggregate !== null ? dataAggregate.count : 0}
          />
        )}
      </ContainerCol>
    </MainContainer>
  );
}

export default DashboardPage;

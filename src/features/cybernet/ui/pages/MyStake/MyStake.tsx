import { Helmet } from 'react-helmet';
import Display from 'src/components/containerGradient/Display/Display';
import DisplayTitle from 'src/components/containerGradient/DisplayTitle/DisplayTitle';
import IconsNumber from 'src/components/IconsNumber/IconsNumber';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import useCurrentAccountStake from '../../hooks/useCurrentAccountStake';
import DelegatesTable from '../Delegates/DelegatesTable/DelegatesTable';
import { HeaderItem } from '../Subnet/SubnetHeader/SubnetHeader';

function MyStake() {
  const { loading, error, data } = useCurrentAccountStake();

  useAdviserTexts({
    isLoading: loading,
    error: error?.message,
    defaultText: 'my learner',
  });

  let length = 0;

  const totalStake =
    data?.reduce((acc, { stake }) => {
      if (stake > 0) {
        length += 1;
      }

      return acc + stake;
    }, 0) || 0;

  return (
    <>
      <Helmet>
        <title>my learner | cyb</title>
      </Helmet>

      <Display>
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
          }}
        >
          <HeaderItem title="mentors" content={length} />

          <HeaderItem
            title="total stake"
            content={<IconsNumber value={totalStake} type="pussy" />}
          />
        </div>
      </Display>
      <Display noPadding title={<DisplayTitle title="My stake" />}>
        <DelegatesTable />
      </Display>
    </>
  );
}

export default MyStake;

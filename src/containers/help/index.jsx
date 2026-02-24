import { connect } from 'react-redux';
import { MainContainer } from 'src/components';
import useSetActiveAddress from '../../hooks/useSetActiveAddress';
import BannerHelp from './BannerHelp';

function Help({ defaultAccount }) {
  const { addressActive } = useSetActiveAddress(defaultAccount);

  return (
    <MainContainer>
      <BannerHelp addressActive={addressActive} />
    </MainContainer>
  );
}

const mapStateToProps = (store) => {
  return {
    defaultAccount: store.pocket.defaultAccount,
  };
};

export default connect(mapStateToProps)(Help);

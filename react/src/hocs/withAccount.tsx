import { useAppSelector } from 'src/redux/hooks';

const withAccount = (Component) => (props: any) => {
  const { defaultAccount } = useAppSelector((state) => state.pocket);

  return <Component {...props} account={defaultAccount} />;
};

export default withAccount;

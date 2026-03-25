import { CardStatisics, Dots, LinkWindow } from '../../components';
import { formatNumber } from '../../utils/utils';

function InfoTokens({ data, selectedTokens, linkSupply, linkPrice, titlePrice }) {
  return (
    <>
      {linkSupply ? (
        <LinkWindow to={linkSupply}>
          <CardStatisics
            title={`${selectedTokens} supply`}
            value={data.loading ? <Dots /> : formatNumber(data.supply)}
            link
          />
        </LinkWindow>
      ) : (
        <CardStatisics
          title={`${selectedTokens} supply`}
          value={data.loading ? <Dots /> : formatNumber(data.supply)}
        />
      )}
      {linkPrice ? (
        <LinkWindow to={linkPrice}>
          <CardStatisics
            title={titlePrice || `price of ${selectedTokens} in ETH`}
            value={data.loading ? <Dots /> : formatNumber(Math.floor(data.price * 1000) / 1000)}
            link
          />
        </LinkWindow>
      ) : (
        <CardStatisics
          title={titlePrice || `price of ${selectedTokens} in ETH`}
          value={data.loading ? <Dots /> : formatNumber(Math.floor(data.price * 1000) / 1000)}
        />
      )}
      <CardStatisics
        title={`${selectedTokens} cap in ETH`}
        value={data.loading ? <Dots /> : formatNumber(data.cap)}
      />
    </>
  );
}

export default InfoTokens;

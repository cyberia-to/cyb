import { Cid } from 'src/components';
import Display from 'src/components/containerGradient/Display/Display';
import IconsNumber from 'src/components/IconsNumber/IconsNumber';
import { AvataImgIpfs } from 'src/containers/portal/components/avataIpfs';
import useCybernetTexts from '../../../useCybernetTexts';
import { useSubnet } from '../subnet.context';
import styles from './SubnetHeader.module.scss';

function Item({ title, content }) {
  return (
    <div className={styles.item}>
      <h6>{title}</h6>
      {content}
    </div>
  );
}

export const HeaderItem = Item;

function SubnetHeader() {
  const { subnetQuery, neuronsQuery } = useSubnet();
  const { getText } = useCybernetTexts();

  const metadata = subnetQuery.data?.metadata || {};

  // fix
  if (!metadata?.name) {
    return null;
  }
  if (!subnetQuery.data) {
    return null;
  }

  const totalNeuronsStake =
    neuronsQuery.data?.reduce(
      (acc, neuron) => acc + neuron.stake.reduce((acc, b) => acc + b[1], 0),
      0
    ) || 0;

  const {
    netuid,
    difficulty,
    tempo,
    max_allowed_validators: maxAllowedValidators,
  } = subnetQuery.data;

  const { logo, description, name } = metadata;

  return (
    <Display>
      <div className={styles.wrapper}>
        <Item
          title={`${getText('subnetwork')} №${netuid}`}
          content={<span className={styles.name}>{name}</span>}
        />
        <Item title={`max ${getText('validator', true)}`} content={maxAllowedValidators} />
        <Item title="difficulty" content={difficulty} />
        <Item title="tempo" content={tempo} />
        <div className={styles.logo}>
          <AvataImgIpfs cidAvatar={logo} />
        </div>
        <Cid cid={description}>description</Cid>
        <Item
          title="teach power"
          content={<IconsNumber value={totalNeuronsStake} type="pussy" />}
        />
      </div>
    </Display>
  );
}

export default SubnetHeader;

import ButtonsGroup from 'src/components/buttons/ButtonsGroup/ButtonsGroup';
import { LinksTypeFilter } from 'src/containers/Search/types';
import styles from './Links.module.scss';

type Props = {
  to: number;
  from: number;
  value: LinksTypeFilter;
  tooltip: {
    from: string;
    to: string;
    particle: string;
  };
  onChange: () => void;
};

const tooltipDefault: Props['tooltip'] = {
  from: 'show only outcoming particles - answers',
  to: 'show only incoming particles - asks',
  particle: 'show all particles',
};

function Links({ to = 0, from = 0, value, tooltip = tooltipDefault, onChange }: Props) {
  return (
    <div className={styles.links}>
      <ButtonsGroup
        type="radio"
        onChange={onChange}
        items={[
          {
            label: String(to),
            name: LinksTypeFilter.to,
            checked: value === LinksTypeFilter.to,
            tooltip: tooltip.to,
          },
          {
            label: <>&rarr;</>,
            disabled: true,
          },
          {
            label: <span />,
            name: LinksTypeFilter.all,
            checked: value === LinksTypeFilter.all,
            tooltip: tooltip.particle,
          },
          {
            label: <>&rarr;</>,
            disabled: true,
          },
          {
            label: String(from),
            name: LinksTypeFilter.from,
            checked: value === LinksTypeFilter.from,
            tooltip: tooltip.from,
          },
        ]}
      />
    </div>
  );
}

export default Links;

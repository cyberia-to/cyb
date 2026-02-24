import { useState } from 'react';
import { Tooltip } from 'src/components';
import ButtonsGroup from 'src/components/buttons/ButtonsGroup/ButtonsGroup';
import Links from 'src/components/search/Spark/Meta/Links/Links';
import { AvataImgIpfs } from 'src/containers/portal/components/avataIpfs';
import useCurrentPassport from 'src/features/passport/hooks/useCurrentPassport';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import { AccountInput } from 'src/pages/teleport/components/Inputs';
import { initialContentTypeFilterState } from '../constants';
import { LinksTypeFilter, SortBy } from '../types';
import styles from './Filters.module.scss';

enum NeuronFilterType {
  me = 'me',
  all = 'all',
  neuron = 'neuron',
}

// TODO: move to ipfs config, global
export const contentTypeConfig = {
  text: {
    label: '📄',
    tooltip: 'show only articles with text',
  },
  image: {
    label: '🖼️',
    tooltip: 'show only images',
  },
  video: {
    label: '🎞️',
    tooltip: 'show only video content',
  },
  pdf: {
    label: '📑',
    tooltip: 'show only pdf files',
  },
  link: {
    label: '🔗',
    tooltip: 'show only particles with links',
  },
  audio: {
    label: '🎧',
    tooltip: 'show only audio content',
  },
};

const sortConfig = {
  [SortBy.rank]: {
    label: '⭐',
    tooltip: 'sort particles by cyberrank',
  },
  [SortBy.date]: {
    label: '📅',
    tooltip: 'sort particles by date of creation',
  },
  // [SortBy.popular]: {
  //   label: '🔥',
  //   tooltip: '',
  // },
  // [SortBy.mine]: {
  //   label: '👤',
  //   tooltip: '',
  // },
};

type Props = {
  linksFilter: LinksTypeFilter;

  neuronFilter: {
    value: string;
    setValue: (address: string | null) => void;
  };
};

function Filters({
  filters,
  setFilters,
  filter2,
  setFilter2,
  linksFilter,
  setLinksFilter,
  total,
  contentType,
  neuronFilter,
}: Props) {
  const [isNeuronChooserOpened, setNeuronChooserOpened] = useState(false);

  const currentAddress = useCurrentAddress();
  const currentPassport = useCurrentPassport();
  const { value: neuron, setValue: setNeuron } = neuronFilter;

  return (
    <>
      <header className={styles.header}>
        <div>
          <ButtonsGroup
            type="checkbox"
            onChange={(filter: typeof initialContentTypeFilterState & 'all') => {
              if (filter === 'all') {
                setFilters(initialContentTypeFilterState);
                return;
              }

              setFilters((filters) => ({
                ...initialContentTypeFilterState,
                [filter]: !filters[filter],
              }));
            }}
            items={[
              {
                label: 'all',
                checked: !Object.values(filters).some((filter) => filter),
              },
            ].concat(
              Object.keys(filters)
                .map((filter) => {
                  if (!Object.values(contentType).includes(filter)) {
                    return null;
                  }

                  return {
                    label: contentTypeConfig[filter].label,
                    name: filter,
                    checked: filters[filter],
                    tooltip: contentTypeConfig[filter].tooltip,
                  };
                })
                .filter((item) => !!item)
            )}
          />
        </div>

        <ButtonsGroup
          type="radio"
          items={Object.values(SortBy).map((sortType) => {
            return {
              label: sortConfig[sortType].label,
              disabled:
                // sortType === SortBy.mine ||
                // sortType === SortBy.popular ||
                sortType === SortBy.rank && neuron,
              name: sortType,
              checked: filter2 === sortType,
              tooltip: sortConfig[sortType].tooltip,
            };
          })}
          onChange={(sortType: SortBy) => {
            setFilter2(sortType);
            localStorage.setItem('search-sort', sortType);
          }}
        />

        <ButtonsGroup
          type="checkbox"
          items={[
            {
              label: (
                <AvataImgIpfs
                  style={{ height: 18, width: 18 }}
                  cidAvatar={currentPassport?.data?.extension.avatar}
                />
              ),
              name: NeuronFilterType.me,
              checked: neuron === currentAddress,
              tooltip: 'show only particles from my neuron',
            },
            {
              label: '🌏',
              name: NeuronFilterType.all,
              checked: !neuron,
              tooltip: 'show all particles',
            },
            {
              label: '👤',
              name: NeuronFilterType.neuron,
              checked: (!!neuron && neuron !== currentAddress) || isNeuronChooserOpened,
              tooltip: 'show only particles from this neuron',
            },
          ]}
          onChange={(name) => {
            let value;
            let value2: typeof isNeuronChooserOpened;
            switch (name) {
              case NeuronFilterType.all:
                value = null;
                break;

              case NeuronFilterType.me:
                value = neuron === currentAddress ? null : currentAddress;
                break;

              case NeuronFilterType.neuron:
                value = null;
                value2 = !isNeuronChooserOpened;
                break;

              default:
                break;
            }

            if (name !== NeuronFilterType.neuron && isNeuronChooserOpened) {
              value2 = false;
            }

            setNeuron(value || null);
            setNeuronChooserOpened(value2 || false);
          }}
        />

        <Links
          to={total.to}
          from={total.from}
          value={linksFilter}
          onChange={(val: LinksTypeFilter) => {
            setLinksFilter(val);
          }}
        />

        <Tooltip tooltip="total particles in result" placement="bottom">
          <div className={styles.total}>
            <span>
              {(() => {
                switch (linksFilter) {
                  case LinksTypeFilter.all:
                    return total.to + total.from;

                  case LinksTypeFilter.to:
                    return total.to;
                  default:
                    return total.from;
                }
              })()}
            </span>{' '}
            particles
          </div>
        </Tooltip>
      </header>

      {isNeuronChooserOpened && (
        <div className={styles.neuronFilter}>
          <AccountInput
            title="choose neuron for filtering"
            recipient={neuron}
            setRecipient={(address) => {
              if (address) {
                setNeuronChooserOpened(false);
              }

              setNeuron(address || null);
            }}
          />
        </div>
      )}
    </>
  );
}

export default Filters;

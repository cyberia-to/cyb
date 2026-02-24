import cx from 'classnames';
import Display from 'src/components/containerGradient/Display/Display';
import Loader2 from 'src/components/ui/Loader2';
import { isParticle } from 'src/features/particle/utils';
import { createLLMThread, selectLLMThread } from 'src/features/sense/redux/sense.redux';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { v4 as uuidv4 } from 'uuid';
import { AdviserProps } from '../Sense';
import { Filters } from '../types';
import NewThreadBtn from './NewThreadBtn/NewThreadBtn';
import styles from './SenseList.module.scss';
import SenseListFilters from './SenseListFilters/SenseListFilters';
import SenseListItem from './SenseListItem/SenseListItem';
import SenseListItemContainer from './SenseListItem/SenseListItem.container';

type Props = {
  select: (id: string) => void;
  selected?: string;
  currentFilter: {
    value: any;
    set: (value: any) => void;
  };
} & AdviserProps;

function SenseList({ select, selected, currentFilter }: Props) {
  const dispatch = useAppDispatch();
  const llmThreads = useAppSelector((state) => state.sense.llm.threads);

  const senseList = useAppSelector((store) => store.sense.list);

  let items = senseList.data || [];

  const filter = currentFilter.value;

  if (filter !== Filters.All) {
    items = items.filter((item) => {
      if (filter === Filters.LLM) {
        return llmThreads.some((thread) => thread.id === item);
      }
      const particle = isParticle(item);
      return (
        particle === (filter === Filters.Particle) || !particle === (filter === Filters.Neuron)
      );
    });
  }

  if (filter === Filters.LLM) {
    items = llmThreads.map((thread) => thread.id);
  }

  function getFilterText(filter: Filters) {
    switch (filter) {
      case Filters.Neuron:
        return 'neuron';

      case Filters.Particle:
        return 'particle';

      default:
        return '';
    }
  }

  const handleNewThread = () => {
    const newThreadId = uuidv4();
    dispatch(createLLMThread({ id: newThreadId }));
    dispatch(selectLLMThread({ id: newThreadId }));
    select(newThreadId);
    currentFilter.set(Filters.LLM);
  };

  return (
    <div className={styles.wrapper}>
      <Display noPadding>
        <div className={styles.filters}>
          <SenseListFilters
            selected={filter}
            onChangeFilter={(filter: Filters) => {
              currentFilter.set(filter);
              select(null);

              // Select LLM chat when LLM filter is selected
              if (filter === Filters.LLM) {
                select('llm');
              }
            }}
          />
        </div>

        {filter === Filters.LLM && (
          // <Button onClick={handleNewThread}>New Thread</Button>
          <NewThreadBtn onClick={handleNewThread} />
        )}

        {filter === Filters.LLM ? (
          <ul>
            {[...llmThreads]
              .sort((a, b) => {
                const aThread = llmThreads.find((thread) => thread.id === a.id);
                const bThread = llmThreads.find((thread) => thread.id === b.id);
                return bThread?.dateUpdated - aThread?.dateUpdated;
              })
              .map((thread) => (
                <li
                  key={thread.id}
                  className={cx(styles.item, {
                    [styles.selected]: selected === thread.id,
                  })}
                >
                  <button
                    type="button"
                    onClick={() => {
                      select(thread.id);
                      dispatch(selectLLMThread({ id: thread.id }));
                    }}
                  >
                    <SenseListItem
                      date={thread.dateUpdated}
                      title={thread.title}
                      isLLM
                      content={(() => {
                        const th = llmThreads.find((item) => item.id === thread.id);
                        const l = th?.messages.length;

                        return th?.messages[l - 1]?.text || 'No messages';
                      })()}
                    />
                  </button>
                </li>
              ))}
          </ul>
        ) : senseList.isLoading && !(items.length > 0) ? (
          <div className={styles.center}>
            <Loader2 />
          </div>
        ) : items.length ? (
          <ul>
            {items.map((id) => (
              <li
                key={id}
                className={cx(styles.item, {
                  [styles.selected]: id === selected,
                })}
              >
                <button
                  type="button"
                  onClick={() => {
                    select(id);
                  }}
                >
                  <SenseListItemContainer
                    senseItemId={id}
                    currentChatId={selected} // **Ensured currentChatId was passed**
                  />
                </button>
              </li>
            ))}
          </ul>
        ) : (
          <p className={styles.center}>no {getFilterText(filter)} chats</p>
        )}
      </Display>
    </div>
  );
}

export default SenseList;

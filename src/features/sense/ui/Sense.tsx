import cx from 'classnames';
import { useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useBackend } from 'src/contexts/backend/backend';
import { useAdviser } from 'src/features/adviser/context';
import { getSenseChat, getSenseList } from 'src/features/sense/redux/sense.redux';
import SenseList from 'src/features/sense/ui/SenseList/SenseList';
import SenseViewer from 'src/features/sense/ui/SenseViewer/SenseViewer';
import { useRobotContext } from 'src/pages/robot/robot.context';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { convertTimestampToString } from 'src/utils/date';
import ActionBar from './ActionBar/ActionBar';
import ActionBarLLM from './ActionBar/ActionBarLLM';
import styles from './Sense.module.scss';
import { Filters } from './types';

export type AdviserProps = {
  adviser: {
    setLoading: (isLoading: boolean) => void;
    setError: (error: string) => void;
    setAdviserText: (text: string) => void;
  };
};

function Sense({ urlSenseId }: { urlSenseId?: string }) {
  const { senseId: paramSenseId } = useParams<{
    senseId: string;
  }>();
  const { isOwner } = useRobotContext();

  const navigate = useNavigate();

  const [selected, setSelected] = useState<string | undefined>(urlSenseId);

  // **Removed null from the type**
  // update state asap
  if (urlSenseId !== selected) {
    setSelected(urlSenseId);
  }

  const dispatch = useAppDispatch();
  const { senseApi } = useBackend();

  // maybe move to another component
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>();
  const [adviserText, setAdviserText] = useState('');

  const [currentFilter, setCurrentFilter] = useState(Filters.All);

  const currentThreadId = useAppSelector((state) => {
    const { llm } = state.sense;
    // console.log(llm);
    return llm.currentThreadId;
  });

  // if (isLLMFilter && !currentThreadId && selected) {
  //   setSelected(null);
  // }

  useEffect(() => {
    if (!selected || !senseApi) {
      return;
    }

    dispatch(
      getSenseChat({
        id: selected,
        senseApi,
      })
    );
  }, [dispatch, selected, senseApi]);

  const syncState = useAppSelector((state) => state.backend.syncState);

  const { setAdviser } = useAdviser();

  useEffect(() => {
    let text;
    let color;

    if (error) {
      color = 'red';
      text = error;
    } else if (loading || syncState.inProgress) {
      color = 'yellow';
      text = loading ? (
        'loading...'
      ) : (
        <p>
          syncing txs data <br />
          {!syncState.initialSyncDone && syncState.inProgress
            ? `${syncState.message} (remaining: ${
                syncState.totalEstimatedTime > -1
                  ? convertTimestampToString(syncState.totalEstimatedTime)
                  : '???'
              })...`
            : ''}
        </p>
      );
    } else {
      text = 'welcome to sense 🧬';
    }
    setAdviser(adviserText || text, error ? 'red' : color);
  }, [setAdviser, loading, error, adviserText, syncState]);

  //  seems use context
  const adviserProps = {
    setLoading: (isLoading: boolean) => setLoading(isLoading),
    setError: (error: string) => setError(error),
    setAdviserText: (text: string) => setAdviserText(text),
  };

  useEffect(() => {
    if (!senseApi) {
      return;
    }

    dispatch(getSenseList(senseApi));
  }, [dispatch, senseApi]);

  function update() {
    // dispatch(getSenseList(senseApi));
    // dispatch(
    //   getSenseChat({
    //     id: selected,
    //     senseApi,
    //   })
    // );
  }

  // useEffect(() => {
  //   if (isLLMFilter && !currentThreadId) {
  //     // Create a new thread when LLM filter is selected and no thread is active
  //     const newThreadId = uuidv4();
  //     dispatch(createLLMThread({ id: newThreadId }));
  //     dispatch(selectLLMThread({ id: newThreadId }));
  //     setSelected(newThreadId);
  //   }
  // }, [isLLMFilter, currentThreadId, dispatch]);

  const isLLMFilter = currentFilter === Filters.LLM;

  return (
    <>
      <div className={cx(styles.wrapper, { [styles.NotOwner]: !isOwner })}>
        {isOwner && (
          <SenseList
            select={(id: string) => {
              setSelected(id);
              // Navigate only if not LLM chat
              if (id !== 'llm') {
                if (!paramSenseId) {
                  navigate(`./${id}`);
                } else {
                  navigate(`../${id}`, {
                    relative: 'path',
                  });
                }
              }
            }}
            selected={selected}
            adviser={adviserProps}
            currentFilter={{
              value: currentFilter,
              set: setCurrentFilter,
            }}
          />
        )}
        <SenseViewer selected={selected} isLLMFilter={isLLMFilter} adviser={adviserProps} />
      </div>

      {isLLMFilter && currentThreadId ? (
        <ActionBarLLM />
      ) : (
        selected && <ActionBar id={selected} adviser={adviserProps} update={update} />
      )}
    </>
  );
}

export default Sense;

import { Pane } from '@cybercongress/gravity';
import { useEffect, useState } from 'react';
import { useDevice } from 'src/contexts/device';
import { useQueryClient } from 'src/contexts/queryClient';
import { getRankGrade } from '../../utils/search/utils';
import { coinDecimals, exponentialToDecimal, timeSince } from '../../utils/utils';
import ContentItem from '../ContentItem/contentItem';
import Rank from '../Rank/rank';

function TimeAgo({ timeAgoInMS }) {
  return (
    <Pane
      className="time-discussion rank-contentItem"
      position="absolute"
      right="-10px"
      fontSize={12}
      whiteSpace="nowrap"
      top="50%"
      transform="translate(0, -50%)"
      marginTop="-5px"
    >
      {timeSince(timeAgoInMS)} ago
    </Pane>
  );
}

const initialState = {
  rank: 'n/a',
  grade: { from: 'n/a', to: 'n/a', value: 'n/a' },
};

function SearchSnippet({ cid, data, onClickRank }) {
  const queryClient = useQueryClient();
  const [rankInfo, setRankInfo] = useState(initialState);
  const { isMobile: mobile } = useDevice();

  useEffect(() => {
    const getRank = async () => {
      if ((data.rank === undefined || data.rank === null) && queryClient) {
        // use useRank hook
        const response = await queryClient.rank(cid);

        const rank = coinDecimals(parseFloat(response.rank));
        const rankData = {
          rank: exponentialToDecimal(rank.toPrecision(3)),
          grade: getRankGrade(rank),
        };
        setRankInfo(rankData);
      }
    };
    getRank();
  }, [data, queryClient, cid]);

  let timeAgoInMS = null;

  if (data.time) {
    const d = new Date();
    const time = Date.parse(d) - Date.parse(data.time);
    if (time > 0) {
      timeAgoInMS = time;
    }
  }
  if (data.timestamp) {
    const d = new Date();
    const time = Date.parse(d) - Date.parse(data.timestamp);
    if (time > 0) {
      timeAgoInMS = time;
    }
  }

  return (
    <Pane
      position="relative"
      className="hover-rank"
      display="flex"
      alignItems="center"
      marginBottom="10px"
    >
      {!mobile && (
        <Pane className="time-discussion rank-contentItem" position="absolute">
          <Rank
            hash={cid}
            rank={rankInfo.rank}
            grade={rankInfo.grade}
            onClick={() => onClickRank(cid)}
          />
        </Pane>
      )}
      <ContentItem cid={cid} item={data} className="contentItem" grade={rankInfo.grade} />
      {timeAgoInMS !== null && <TimeAgo timeAgoInMS={timeAgoInMS} />}
    </Pane>
  );
}

export default SearchSnippet;

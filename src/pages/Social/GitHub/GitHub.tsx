import { useQuery } from '@tanstack/react-query';
import axios from 'axios';
import { LinkWindow, Tooltip } from 'src/components';
import { formatNumber } from 'src/utils/utils';
import useMediaQuery from '../../../hooks/useMediaQuery';
import share from '../../../image/share.svg';
import star from '../../../image/star-reg.svg';
import styles from './GitHub.module.scss';
import githubIcon from './github-white-icon.svg';

const getStargazersGitHub = async () => {
  try {
    const response = await axios({
      method: 'get',
      url: 'https://api.github.com/repos/cybercongress/cyb',
    });

    return response.data;
  } catch (_e) {
    return null;
  }
};

export function StargazersCountGH() {
  const { data } = useQuery({
    queryKey: ['stargazers_count'],
    queryFn: async () => {
      const responce = await getStargazersGitHub();

      if (responce !== null) {
        return responce;
      }

      return undefined;
    },
  });

  return <div>{data !== undefined ? formatNumber(data.stargazers_count) : '...'}</div>;
}

export function GitHub() {
  const _mediaQuery = useMediaQuery('(min-width: 768px)');

  if (true) {
    return (
      <div
        id="github-bar"
        className={styles.wrapper}
        style={{
          // position: 'fixed',
          right: '0',
          bottom: 0,
          // margin: '0px 20px 28px 0px',
          fontSize: '16px',
          // background: '#000c',
          zIndex: 4,
        }}
      >
        <LinkWindow to="https://github.com/cybercongress/cyb-ts">
          <img alt="github" style={{ width: 30, height: 30 }} src={githubIcon} />
          <span>app</span>
        </LinkWindow>

        <LinkWindow to="https://github.com/cybercongress">
          <img alt="github" style={{ width: 30, height: 30 }} src={githubIcon} />

          <span>congress</span>
        </LinkWindow>
      </div>
    );
  }

  return (
    <div
      id="github-bar"
      style={{
        position: 'fixed',
        right: '0',
        bottom: 0,
        margin: '0px 20px 28px 0px',
        fontSize: '14px',
        background: '#000c',
        zIndex: 4,
      }}
    >
      <Tooltip
        placement="left"
        hideBorder
        tooltip={
          <div
            id="github-bar"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '20px',
              background: '#000c',
              padding: 5,
              marginRight: 15,
            }}
          >
            <LinkWindow to="https://github.com/cybercongress/cyber/blob/master/join.md">
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  color: '#36D6AE',
                  whiteSpace: 'nowrap',
                }}
              >
                join devs <img alt="share" style={{ width: 20, height: 20 }} src={share} />
              </div>
            </LinkWindow>
            <LinkWindow to="https://github.com/cybercongress/cyb/issues/new">
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  color: '#36D6AE',
                  whiteSpace: 'nowrap',
                }}
              >
                submit issue <img alt="share" style={{ width: 20, height: 20 }} src={share} />
              </div>
            </LinkWindow>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                height: '30px',
              }}
            >
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '5px',
                  backgroundColor: 'rgb(40, 40, 40)',
                  padding: '5px',
                  height: '100%',
                  borderRadius: '5px 0px 0px 5px',
                  borderRight: '1px solid #cccc',
                }}
              >
                <img alt="star" style={{ width: 20, height: 20 }} src={star} />
                Star
              </div>
              <div
                style={{
                  backgroundColor: 'rgb(17, 20, 30)',
                  padding: '5px',
                  height: '100%',
                  display: 'flex',
                  alignItems: 'center',
                  borderRadius: '0px 5px 5px 0',
                }}
              >
                {' '}
                <StargazersCountGH />
              </div>
            </div>
          </div>
        }
      >
        <LinkWindow to="https://github.com/cybercongress">
          <div>
            <img alt="github" style={{ width: 30, height: 30 }} src={github} />
          </div>
        </LinkWindow>
      </Tooltip>
    </div>
  );
}

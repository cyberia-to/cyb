import { Pane } from '@cybercongress/gravity';
import cx from 'classnames';
import React from 'react';
import LocalizedStrings from 'react-localization';
import { Button } from 'src/components';
import { i18n } from '../../i18n/en';
import styles from './story.module.scss';

const mp3 = require('./starwars.mp3');
const cyberImg = require('../../image/cyber.png');
const cybImg = require('../../image/cyb.svg');

const T = new LocalizedStrings(i18n);

class Story extends React.Component {
  constructor(props) {
    super(props);
    let story = false;
    const localStorageStory = localStorage.getItem('story');
    if (localStorageStory !== null) {
      story = localStorageStory;
    }
    this.state = {
      btnPlay: false,
      animated: false,
      end: false,
      cyber: false,
      cyb: false,
      story,
    };
  }

  componentDidMount() {
    this.setState({
      btnPlay: true,
    });
    this.chekKeydown();
  }

  chekKeydown = () => {
    document.addEventListener('keydown', (event) => {
      if (event.code === 'Space') {
        this.swapStory();
      }
    });
  };

  startTimer = () => {
    this.setState({
      btnPlay: false,
    });
    setTimeout(() => {
      console.log('sdf');
      this.setState({
        animated: true,
      });
    }, 6000);

    setTimeout(() => {
      this.setState({
        cyber: true,
      });
    }, 34000);

    setTimeout(() => {
      this.setState({
        cyb: true,
      });
    }, 54500);

    setTimeout(() => {
      this.setState({
        end: true,
      });
    }, 65000);

    setTimeout(() => {
      this.swapStory();
    }, 70000);
  };

  // eslint-disable-next-line class-methods-use-this
  audio = () => {
    const sound = document.getElementById('sound');
    sound.volume = 0.2;
    const fade = setInterval(() => {
      if (sound.currentTime >= 55 && sound.volume >= 0.1) {
        sound.volume -= 0.002;
      }
      if (sound.currentTime >= 10 && sound.volume <= 0.1) {
        sound.volume = 0.0;
      }
      if (sound.volume === 0.0) {
        clearInterval(fade);
      }
    }, 200);
  };

  Play = () => {
    document.getElementById('sound').play();
    this.audio();
    this.startTimer();
  };

  swapStory = () => {
    const { history } = this.props;
    localStorage.setItem('story', JSON.stringify(true));
    // navigate('/superintelligence');
  };

  render() {
    const { animated, end, cyber, cyb, story, btnPlay } = this.state;

    return (
      <div>
        <div className={styles.story} style={{ opacity: `${end ? 0 : 1}` }}>
          {!story && (
            <div
              style={{ display: 'flex', justifyContent: 'space-between' }}
              className={styles.containerStory}
            >
              <Pane
                width={50}
                // height={50}
                position="relative"
                display="flex"
                align-items="flex-end"
              >
                {cyber && <img style={{ width: 'inherit' }} alt="cyb" src={cyberImg} />}
              </Pane>
              <Pane
                width={50}
                // height={50}
                position="relative"
                display="flex"
                align-items="flex-end"
              >
                {cyb && <img style={{ width: 'inherit' }} alt="cyb" src={cybImg} />}
              </Pane>
            </div>
          )}
          <section
            className={cx(styles.section, styles.title)}
            id="title"
            style={{
              opacity: `${animated ? 0 : 1}`,
              transition: 'opacity 0.3s',
            }}
          >
            <p>A long time ago somewhere in Cosmos ...</p>
          </section>

          <section className={cx(styles.content, styles.section)}>
            <div id="text" className={`${animated ? 'animated' : ''}`}>
              <p style={{ textAlign: 'center' }}>{T.story.episode}</p>
              <p style={{ textAlign: 'center' }}>{T.story.header}</p>
              <p>{T.story.itIsAPeriod}</p>
              <p>{T.story.resistingRebel}</p>
              <p>{T.story.asTheyBegin}</p>
            </div>
          </section>
          {/* eslint-disable-next-line jsx-a11y/media-has-caption */}
          <audio id="sound" preload="auto">
            <source src={mp3} type="audio/mpeg" />
          </audio>
        </div>
        {btnPlay && <Button text="Play story" onClick={this.Play} style={{ zIndex: 3 }} />}
      </div>
    );
  }
}

export default Story;

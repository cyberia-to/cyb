/* eslint-disable */

// import Room from 'ipfs-pubsub-room';
import { ActionBar, Pane, Text } from '@cybercongress/gravity';
import React from 'react';
import TextareaAutosize from 'react-textarea-autosize';
import { ActionBarContentText, NoItems } from '../../components';
import { trimString } from '../../utils/utils';

const imgSend = require('../../image/paper-plane-outline.svg');

const _test = [
  { name: 'anonymous coward', text: 'cvcvx1' },
  {
    name: 'anonymous1',
    text: 'Trollginator there is enough BTC to go around man lol and keep my call in mind, 48H from now come back and check in',
  },
  { name: 'anonymous2', text: 'gi anonymous' },
  { name: 'anonymous3', text: 'dskx234ewrd' },
  { name: 'anonymou4', text: 'skdfksdk fl123klxd' },
];

const _TOPIC = 'cyberChat';

const _addr = '/ip4/159.89.24.179/tcp/4001/p2p/QmZBfqaL2L92rrTWR2Cdmor3R3EBLaoYzeVLEEwE3AJmWe';

function _repo() {
  // returns random repo path to have different ID's
  // random ID's generated each reload
  return `ipfs/pubsub/${Math.random()}`;
}

function MessageItem({ name = '', text = '' }) {
  return (
    <Pane
      backgroundColor="#03cba029"
      borderRadius="5px"
      paddingX={5}
      paddingY={5}
      marginY={5}
      width="70%"
      marginX="auto"
    >
      {/* <Pane overflow="hidden" whiteSpace="nowrap" textOverflow="ellipsis">
      {name}
    </Pane> */}
      <Pane>
        <Text color="#36d6ae">{trimString(name, 4, 4)}</Text>:{' '}
        <span style={{ whiteSpace: 'pre-wrap' }}>{text}</span>
      </Pane>
    </Pane>
  );
}

function SendMessage({
  onClickSend,
  placeholder = 'message',
  onChangeMessage,
  valueInputMassage = '',
  handleKeyDown,
}) {
  return (
    <Pane width="65%" alignItems="flex-end" display="flex">
      <ActionBarContentText>
        <TextareaAutosize
          value={valueInputMassage}
          maxRows={20}
          style={{
            height: 42,
            width: '100%',
            color: '#fff',
            paddingLeft: '10px',
            borderRadius: '20px',
            textAlign: 'start',
            paddingRight: '35px',
            paddingTop: '10px',
            paddingBottom: '10px',
            marginRight: '15px',
          }}
          className="resize-none minHeightTextarea"
          onChange={onChangeMessage}
          placeholder={placeholder}
          onFocus={(e) => {
            e.target.placeholder = '';
          }}
          onBlur={(e) => {
            e.target.placeholder = placeholder;
          }}
          onKeyDown={(e) => handleKeyDown(e)}
        />
      </ActionBarContentText>
      <button
        className="container-buttonIcon"
        type="button"
        onClick={onClickSend}
        style={{ height: 25 }}
        disabled={/^\s*$/.test(valueInputMassage)}
      >
        <img src={imgSend} alt="edit" style={{ width: 25, height: 25 }} />
      </button>
    </Pane>
  );
}

class TrollBoxx extends React.PureComponent {
  constructor(props) {
    super(props);
    this.state = {
      valueInputMassage: '',
      messages: [],
      name: window.localStorage.getItem('name') || 'anonymous coward',
      room: null,
    };
    this.chatContainer = React.createRef();
  }

  async componentDidMount() {
    const { node } = this.context;
    if (node !== null) {
      // await this.init();
    }
    this.scrollToBottom();
  }

  componentDidUpdate(prevProps) {
    const { node } = this.context;
    if (node !== prevProps.node) {
      // this.init();
    }
    this.scrollToBottom();
  }

  scrollToBottom = () => {
    const { messages } = this.state;
    if (messages.length > 0) {
      this.chatContainer.current.scrollIntoView({ behavior: 'smooth' });
    }
  };

  init = async () => {
    // const { node, ipfsId } = this.props;
    // const roomIpfs = new Room(node, TOPIC);
    // this.setState({
    //   room: roomIpfs,
    // });
    // console.log('roomIpfs', roomIpfs);
    // // setRoom(roomIpfs);
    // roomIpfs.on('peer joined', (peer) => console.log(`peer ${peer} joined`));
    // roomIpfs.on('peer left', (peer) => console.log(`peer ${peer} left`));
    // // roomIpfs.on('peer joined', (peer) =>
    // //   roomIpfs.sendTo(peer, `Hello ${peer}`)
    // // );
    // const peers = roomIpfs.getPeers();
    // console.log(peers);
    // roomIpfs.on('message', (message) => {
    //   console.log('message', message);
    //   console.log('message', message.data.toString());
    //   const { from } = message;
    //   const data = message.data.toString();
    //   let msg = {};
    //   // if (data.indexOf('{"name":') !== -1) {
    //   //   const msgObj = JSON.parse(data);
    //   //   msg = { ...msgObj };
    //   // } else {
    //   msg = {
    //     name: from,
    //     text: data,
    //   };
    //   // }
    //   // Update Messages
    //   this.updatedMessages(msg);
    // });
  };

  // updatedMessages = (msg = {}) => {
  //   const { messages } = this.state;

  //   if (Object.keys(msg).length > 0) {
  //     if (messages.length > 0) {
  //       const updatedMessages = [...messages, msg].slice(-1000);
  //       this.setState({ messages: updatedMessages });
  //     } else {
  //       const updatedMessages = [msg];
  //       this.setState({ messages: updatedMessages });
  //     }
  //   }
  // };

  onClickSend = async () => {
    const { room, valueInputMassage } = this.state;
    if (room !== null) {
      try {
        // JSON.stringify(
        room.broadcast(valueInputMassage);
        this.setState({
          valueInputMassage: '',
        });
      } catch (error) {
        console.error('Failed to publish', error);
      }
    }
  };

  nameChange = () => {
    const { valueInputName } = this.state;

    window.localStorage.setItem('name', valueInputName);
    this.setState({
      name: valueInputName,
    });
  };

  onChangeMessage = (e) => {
    const { value } = e.target;
    this.setState({
      valueInputMassage: value,
    });
  };

  handleKeyDown = (event) => {
    const { valueInputMassage, room } = this.state;

    if (event.keyCode === 13 && !event.shiftKey && !/^\s*$/.test(valueInputMassage)) {
      try {
        // JSON.stringify(
        room.broadcast(valueInputMassage);
        event.preventDefault();
        this.setState({
          valueInputMassage: '',
        });
      } catch (error) {
        console.error('Failed to publish', error);
      }
    }
  };

  render() {
    const { name, valueInputName, valueInputMassage, messages } = this.state;
    console.log('valueInputMassage', valueInputMassage, valueInputMassage.length);

    console.log('messages', messages);

    return (
      <>
        <main className="block-body" style={{ paddingTop: 30, alignItems: 'center' }}>
          <Pane width="90%" marginX="auto" marginY={0} display="flex" flexDirection="column">
            <div className="container-contentItem" style={{ width: '100%' }}>
              {/* <Pane height="500px" overflowY="scroll" paddingRight={5}> */}
              {messages.length > 0 ? (
                messages.map((item, i) => (
                  <>
                    <MessageItem key={i} name={item.name} text={item.text} />
                    <div ref={this.chatContainer} />
                  </>
                ))
              ) : (
                <NoItems
                  text={
                    <Pane textAlign="center">
                      <Pane marginBottom={5}>No messages.</Pane>
                      <Pane>Write something below to begin</Pane>
                    </Pane>
                  }
                />
              )}
              {/* </Pane> */}
            </div>
          </Pane>
        </main>
        <ActionBar>
          <SendMessage
            valueInputMassage={valueInputMassage}
            onChangeMessage={(e) => this.onChangeMessage(e)}
            onClickSend={() => this.onClickSend()}
            handleKeyDown={this.handleKeyDown}
          />
        </ActionBar>
      </>
    );
  }
}

export default TrollBoxx;

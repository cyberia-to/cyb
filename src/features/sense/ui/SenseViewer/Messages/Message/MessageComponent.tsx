import { LLMAvatar } from 'src/containers/Search/LLMSpark/LLMSpark';
import { LLMMessage } from 'src/features/sense/redux/sense.redux';
import Message from './Message';

interface Props {
  message: LLMMessage;
}

function MessageComponent({ message }: Props) {
  const isUser = message.sender === 'user';

  return (
    <Message
      myMessage={isUser}
      date={message.timestamp}
      content={message.text}
      // from="llm"
      // fromLog
      avatar={!isUser && <LLMAvatar onlyImg />}
    />
  );
}

export default MessageComponent;

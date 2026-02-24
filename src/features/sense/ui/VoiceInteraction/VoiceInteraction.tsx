import { useRef, useState } from 'react';
import { Button } from 'src/components';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { llmRequest } from '../../../../containers/Search/LLMSpark/LLMSpark';
import { addLLMMessageToThread, replaceLastLLMMessageInThread } from '../../redux/sense.redux';
import styles from './VoiceInteraction.module.scss';

function VoiceInteraction() {
  const [isRecording, setIsRecording] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const mediaRecorder = useRef<MediaRecorder | null>(null);
  const audioChunks = useRef<Blob[]>([]);
  const dispatch = useAppDispatch();
  const currentThreadId = useAppSelector((state) => state.sense.llm.currentThreadId);

  // useEffect(() => {
  //   if (!currentThreadId) {
  //     const newThreadId = uuidv4();
  //     dispatch(createLLMThread({ id: newThreadId }));
  //     dispatch(selectLLMThread({ id: newThreadId }));
  //   }
  // }, [currentThreadId, dispatch]);

  const startRecording = async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      mediaRecorder.current = new MediaRecorder(stream);
      audioChunks.current = [];

      mediaRecorder.current.ondataavailable = (event) => {
        audioChunks.current.push(event.data);
      };

      mediaRecorder.current.onstop = handleAudioData;

      mediaRecorder.current.start();
      setIsRecording(true);
      setError(null);
    } catch (error) {
      console.error('Error accessing microphone:', error);
      setError('Error accessing microphone. Please check your permissions.');
    }
  };

  const stopRecording = () => {
    if (mediaRecorder.current && isRecording) {
      mediaRecorder.current.stop();
      setIsRecording(false);
      setIsProcessing(true);
    }
  };

  const handleAudioData = async () => {
    const audioBlob = new Blob(audioChunks.current, { type: 'audio/wav' });
    const formData = new FormData();
    formData.append('file', audioBlob, 'audio.wav');
    formData.append('model', 'whisper-1');

    try {
      const apiKey = process.env.REACT_APP_OPENAI_API_KEY;
      if (!apiKey) {
        throw new Error('OpenAI API key is not set in environment variables');
      }

      const response = await fetch('https://api.openai.com/v1/audio/transcriptions', {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${apiKey}`,
        },
        body: formData,
      });

      if (!response.ok) {
        const errorBody = await response.text();
        throw new Error(`HTTP error! status: ${response.status}, body: ${errorBody}`);
      }

      const data = await response.json();
      const transcription = data.text;

      if (!transcription) {
        throw new Error('Transcription is empty');
      }

      if (currentThreadId) {
        dispatch(
          addLLMMessageToThread({
            threadId: currentThreadId,
            message: {
              text: transcription,
              sender: 'user',
              timestamp: Date.now(),
            },
          })
        );

        const waitingMessage = {
          text: 'Processing...',
          sender: 'llm',
          timestamp: Date.now(),
        };
        dispatch(
          addLLMMessageToThread({
            threadId: currentThreadId,
            message: waitingMessage,
          })
        );

        const aiResponse = await llmRequest(transcription);

        dispatch(
          replaceLastLLMMessageInThread({
            threadId: currentThreadId,
            message: { text: aiResponse, sender: 'llm', timestamp: Date.now() },
          })
        );

        // Text-to-speech for AI response
        const speech = new SpeechSynthesisUtterance(aiResponse);
        window.speechSynthesis.speak(speech);
      } else {
        throw new Error('No current thread ID');
      }
    } catch (error) {
      console.error('Error processing audio:', error);
      setError(`Error: ${error.message}`);
      if (currentThreadId) {
        dispatch(
          addLLMMessageToThread({
            threadId: currentThreadId,
            message: {
              text: `Error: ${error.message}`,
              sender: 'llm',
              timestamp: Date.now(),
            },
          })
        );
      }
    } finally {
      setIsProcessing(false);
    }
  };

  return (
    <div className={styles.voiceInteraction}>
      <Button onClick={isRecording ? stopRecording : startRecording} disabled={isProcessing}>
        {isRecording ? '🎤🛑' : '🎤'}
      </Button>
      {isProcessing && <p>Processing audio...</p>}
      {error && <p className={styles.error}>{error}</p>}
    </div>
  );
}

export default VoiceInteraction;

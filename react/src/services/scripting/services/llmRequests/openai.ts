/* eslint-disable import/prefer-default-export */
/* eslint-disable import/no-unused-modules */

// https://platform.openai.com/docs/models/overview
// gpt-3.5-turbo

type OpenAiMessage = {
  role: 'system' | 'user' | 'assistant';
  content: string;
};

interface OpenAIParams {
  model: string;
  messages: OpenAiMessage[];
  stream?: boolean;
  [key: string]: any;
}

const defaultOpenAIParams: Partial<OpenAIParams> = {
  model: 'gpt-3.5-turbo',
};

export const openAICompletion = async (
  messages: OpenAiMessage[],
  apiKey: string,
  params: Partial<OpenAIParams> = {},
  cb: (s: string) => Promise<void>,
  abortController?: AbortController
): Promise<string> => {
  const body = JSON.stringify({
    messages,
    ...defaultOpenAIParams,
    ...params,
  });

  const headers = {
    'Content-Type': 'application/json',
    Authorization: `Bearer ${apiKey}`,
  };

  const response = await fetch('https://api.openai.com/v1/chat/completions', {
    method: 'POST',
    signal: abortController?.signal,
    headers,
    body,
  });

  if (!params.stream) {
    // Non-streaming request
    const data = await response.json();
    return data.choices[0].message.content;
  }
  // Streaming request
  const reader = response.body?.getReader();
  const decoder = new TextDecoder();
  let result = '';
  let buffer = '';

  if (reader) {
    // eslint-disable-next-line no-constant-condition
    while (true) {
      // eslint-disable-next-line no-await-in-loop
      const { done, value } = await reader.read();
      if (done || abortController?.signal.aborted) {
        break;
      }

      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split('\n');

      // Keep the last partial line in the buffer
      buffer = lines.pop() || '';

      // eslint-disable-next-line no-restricted-syntax
      for (const line of lines) {
        const message = line.replace(/^data: /, '');
        if (message === '[DONE]') {
          return result;
        }
        try {
          const parsed = JSON.parse(message);
          if (parsed.choices && parsed.choices.length > 0) {
            const { content } = parsed.choices[0].delta;
            result += content;
            if (content) {
              // eslint-disable-next-line no-await-in-loop
              await cb(content);
            }
          }
        } catch (error) {
          console.error('Error parsing stream message:', message, error);
        }
      }
    }
  }

  return result;
};

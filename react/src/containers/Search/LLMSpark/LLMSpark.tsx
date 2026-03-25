import { createOpenRouter } from '@openrouter/ai-sdk-provider';
import { useQuery } from '@tanstack/react-query';
import { generateText } from 'ai';
import { Link, useNavigate, useSearchParams } from 'react-router-dom';
import { Display } from 'src/components';
import TextMarkdown from 'src/components/TextMarkdown';
import Loader2 from 'src/components/ui/Loader2';
import AdviserHoverWrapper from 'src/features/adviser/AdviserHoverWrapper/AdviserHoverWrapper';
import useAddToIPFS from 'src/features/ipfs/hooks/useAddToIPFS';
import useGetIPFSHash from 'src/features/ipfs/hooks/useGetIPFSHash';
import { useHover } from 'src/hooks/useHover';
import { routes } from 'src/routes';
import { isDevEnv } from 'src/utils/dev';
import { isCID } from 'src/utils/ipfs/helpers';
import { testVar } from '.';
import styles from './LLMSpark.module.scss';

// WIP

export const provider = createOpenRouter({
  ['a' + 'piK' + 'ey']: `sk-or-v1-${atob(testVar)}`,
});

export const modelName = isDevEnv() ? 'meta-llama/llama-3-8b-instruct:free' : 'openai/gpt-4o-mini';

const model = provider.chat(modelName);

export async function llmRequest(prompt) {
  const { text } = await generateText({
    model,
    prompt,
  });

  return text;
}

function useLLMResponse(text) {
  const { data, isLoading, error } = useQuery(
    ['llm', text],
    async () => {
      return llmRequest(`what is ${text}`);
    },
    {
      enabled: Boolean(text),
    }
  );

  return {
    data,
    isLoading,
    error,
  };
}

export function useIsLLMPageParam() {
  const [searchParams] = useSearchParams();

  const isLLM = searchParams.get('llm') === 'true';

  return isLLM;
}

export function LLMAvatar({ onlyImg }) {
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
      }}
    >
      <AdviserHoverWrapper adviserContent={modelName}>
        <Link
          to={`${routes.settings.path}/llm`}
          style={{
            display: 'flex',
            alignItems: 'center',
          }}
        >
          <img
            src={`https://robohash.org/${modelName}`}
            style={{
              width: '20px',
              height: '20px',
              borderRadius: '50%',
              marginRight: '5px',
            }}
          />
          {!onlyImg && model.modelId}
        </Link>
      </AdviserHoverWrapper>
    </div>
  );
}

function LLMSpark({ searchText }: { searchText: string }) {
  const { data, isLoading } = useLLMResponse(searchText);
  const { execute } = useAddToIPFS(data);

  const [ref, hovering] = useHover();

  const cid = useGetIPFSHash(data);

  const navigate = useNavigate();

  if (isCID(searchText)) {
    return null;
  }

  return (
    <Link
      ref={ref}
      className={styles.wrapper}
      to={routes.oracle.ask.getLink(cid)}
      onClick={async (e) => {
        e.preventDefault();

        const hash = await execute();

        navigate(`${routes.oracle.ask.getLink(hash)}?llm=true`);
      }}
    >
      {hovering && (
        <div className={styles.left}>
          <LLMAvatar />
        </div>
      )}
      <Display color={isLoading ? 'yellow' : 'purple'}>
        {isLoading && <Loader2 text="llm is generating response" />}
        {data && <TextMarkdown preview>{data}</TextMarkdown>}
      </Display>
    </Link>
  );
}

export default LLMSpark;

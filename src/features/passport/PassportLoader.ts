import { Citizenship } from 'src/types/citizenship';
import usePassportContract from './usePassportContract';

function PassportLoader({
  tokenId,
  render,
}: {
  tokenId: string;
  render: (passport: Citizenship) => JSX.Element | null;
}) {
  const { data: passport } = usePassportContract<Citizenship>({
    query: {
      nft_info: {
        token_id: tokenId,
      },
    },
  });

  if (!passport) {
    return null;
  }
  return render(passport);
}

export default PassportLoader;

import { useState } from 'react';
import { Button, Display, DisplayTitle, Input, MainContainer } from 'src/components';
import { FILTERING_CONTRACT } from 'src/contexts/appData';
import useExecuteContractWithWaitAndAdviser from 'src/hooks/contract/useExecuteContractWithWaitAndAdviser';
import useQueryContract from 'src/hooks/contract/useQueryContract';

function Filtering() {
  const [value, setValue] = useState('');
  const { mutate } = useExecuteContractWithWaitAndAdviser({
    contractAddress: FILTERING_CONTRACT,
    query: {
      add_particles: {
        particles: value.split(',').map((item) => item.trim()),
      },
    },
  });

  const query = useQueryContract(FILTERING_CONTRACT, {
    particles: {},
  });

  console.log(value.split(',').map((item) => item.trim()));

  return (
    <MainContainer>
      <Display title={<DisplayTitle title="restricted content" />}>
        <Input
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder="add particles"
          isTextarea
        />

        <Button onClick={mutate}>add particles</Button>
      </Display>

      <Display title={<DisplayTitle title="particles" />}>
        {query.data?.map((item) => (
          <a
            href={`https://gateway.ipfs.cybernode.ai/ipfs/${item[1]}`}
            target="_blank"
            rel="noreferrer"
          >
            {item[1]}
          </a>
          // <ContentIpfs key={item[1]} cid={item[1]} skipCheck />
        ))}
      </Display>
    </MainContainer>
  );
}

export default Filtering;

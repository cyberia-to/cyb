import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Display, DisplayTitle, Input } from 'src/components';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import { routes } from 'src/routes';
import CreateProposalActionBar from './CreateProposalActionBar';

function CreateProposal() {
  const [text, setText] = useState('');
  const [title, setTitle] = useState('');

  useAdviserTexts({
    defaultText: 'Create a proposal',
  });

  return (
    <div>
      <Display title={<DisplayTitle title="Create proposal" />}>
        <Input value={title} onChange={(e) => setTitle(e.target.value)} placeholder="title" />

        <br />

        <Input
          isTextarea
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="description"
        />
      </Display>

      <br />

      <Display>
        <p>only text proposals are supported at the moment</p>

        <br />

        <p>
          if need different, <Link to={routes.social.path}>contact devs</Link> or{' '}
          <Link
            to="https://docs.cosmos.network/v0.50/build/modules/gov#proposal-submission"
            reloadDocument
            target="_blank"
          >
            use CLI
          </Link>
        </p>
      </Display>

      <CreateProposalActionBar
        proposal={{
          title,
          description: text,
        }}
      />
    </div>
  );
}

export default CreateProposal;

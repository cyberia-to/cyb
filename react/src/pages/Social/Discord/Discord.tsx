import { LinkWindow } from 'src/components/link/link';
import discordIcon from './discord-icon.svg';

function Discord() {
  return (
    <LinkWindow to="https://discord.gg/cyber-bostrom-spacepussy">
      <img src={discordIcon} alt="Discord" />
      <span>Discord</span>
    </LinkWindow>
  );
}

export default Discord;

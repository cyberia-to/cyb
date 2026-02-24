import { Link } from 'react-router-dom';
import Tooltip from 'src/components/tooltip/tooltip';
import { Dots } from 'src/components/ui/Dots';
import { routes } from 'src/routes';
import Button from '..';

function NodeIsLoadingButton() {
  return (
    <Tooltip
      tooltip={
        <>
          If long, check <Link to={routes.robot.routes.drive.path}>Drive</Link>
        </>
      }
    >
      <Button disabled>
        Node is loading&nbsp;
        <Dots />
      </Button>
    </Tooltip>
  );
}

export default NodeIsLoadingButton;

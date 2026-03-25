import { Meta } from '@storybook/react';
import ReleaseStatus from './index';

export default {
  component: ReleaseStatus,
  title: 'Features/Gift/ReleaseStatus',
} as Meta;

function Template(args) {
  return <ReleaseStatus {...args} />;
}

export const Default = Template.bind({});
Default.args = {
  status: 'red',
  amountGiftValue: 10,
  progress: 10,
  data: {
    availableRelease: 10,
    released: 100,
    leftRelease: 10000,
  },
  nextRelease: 10,
};

import { forwardRef, useImperativeHandle, useState } from 'react';
import Sign from 'src/pages/Sign/Sign';
import Modal from '../modal/Modal';

export interface SignerModalRef {
  open: () => void;
  close: () => void;
}

const SignerModal = forwardRef<SignerModalRef>((_props, ref) => {
  const [isOpen, setIsOpen] = useState(false);

  useImperativeHandle(ref, () => ({
    open: () => {
      setIsOpen(true);
    },
    close: () => {
      setIsOpen(false);
    },
  }));

  return (
    <Modal style={{ width: '100vw', height: '100vh' }} isOpen={isOpen}>
      <Sign />
    </Modal>
  );
});

SignerModal.displayName = 'SignerModal';

export default SignerModal;

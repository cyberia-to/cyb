import { useRef } from 'react';
import { v4 as uuidv4 } from 'uuid';

function useId() {
  const id = useRef(uuidv4()).current;
  return id;
}

export default useId;

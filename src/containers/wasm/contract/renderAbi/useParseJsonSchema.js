import { useEffect, useState } from 'react';

const useParseJsonSchema = (fileAbi) => {
  const [dataObj, setDataObj] = useState([]);

  useEffect(() => {
    if (fileAbi !== null) {
      const objArr = [];
      const copyObjGlobal = { ...fileAbi };
      delete copyObjGlobal.title;
      if (fileAbi.oneOf) {
        delete copyObjGlobal.oneOf;
        Object.keys(fileAbi.oneOf).forEach((key) =>
          objArr.push({
            ...copyObjGlobal,
            ...fileAbi.oneOf[key],
          })
        );
      } else if (fileAbi.allOf) {
        delete copyObjGlobal.allOf;
        Object.keys(fileAbi.allOf).forEach((key) =>
          objArr.push({
            ...copyObjGlobal,
            ...fileAbi.allOf[key],
          })
        );
      } else {
        objArr.push({
          ...fileAbi,
        });
      }
      setDataObj(objArr);
    } else {
      setDataObj([]);
    }
  }, [fileAbi]);

  return { dataObj };
};

export default useParseJsonSchema;

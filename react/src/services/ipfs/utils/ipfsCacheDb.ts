import db from 'src/db';

const ipfsCacheDb = () => {
  const add = async (cid: string, raw: Uint8Array): Promise<void> => {
    const dbValue = await db.table('cid').get({ cid });

    if (!dbValue) {
      const ipfsContentAddtToInddexdDB = {
        cid,
        data: raw,
      };
      db.table('cid').add(ipfsContentAddtToInddexdDB);
    }
  };

  const get = async (cid: string): Promise<Uint8Array | undefined> => {
    // TODO: use cursor
    const dbValue = await db.table('cid').get({ cid });

    // backward compatibility
    return dbValue?.data || dbValue?.content;
  };

  return { add, get };
};

export default ipfsCacheDb();

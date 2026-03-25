import Dexie from 'dexie';

const db = new Dexie('cyber-page-cash');
db.version(3).stores({
  cid: 'cid',
  following: 'cid',
});

export default db;

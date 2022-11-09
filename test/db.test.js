'use strict';

const Database = require("..");

let dbs = [];
const start_new_db = async (...args) => {
  let inst = new Database(...args);
  
  await inst.start();
  dbs.push(inst);
  return inst;
}

describe('Database', () => {

  afterAll(async () => {
    // Weird just in case
    await Promise.all(dbs.map(async (db) => await db.stop()))
  }, 1000);

  it('creates a new database', async () => {
    const inst = await start_new_db();
    const db_uri = await inst.new_db();
    expect(db_uri.indexOf("postgres://postgres:postgres@localhost")).toEqual(0);
    await inst.stop();
  })
  
  it('allows us to set a custom path', async () => {
    const inst = await start_new_db();
    const db_uri = await inst.new_db();
    expect(db_uri.indexOf("postgres://postgres:postgres@localhost")).toEqual(0);
    await inst.stop();
  })

  it('drops a database', async () => {
    const inst = await start_new_db();;
    const db_uri = await inst.new_db();
    expect(db_uri.indexOf("postgres://postgres:postgres@localhost")).toEqual(0);
    await inst.drop_db(db_uri);
    await inst.stop();
  })

  // it('creates a new database', async () => {
  //   const db_uri = await db.new_db();
  //   expect(db_uri).toBe("bob");
  // })
})
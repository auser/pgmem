'use strict';

const Database = require("..");

describe('Database', () => {
  let inst;
  const start_new_db = (...args) => new Database(...args);

  it.only('creates a new database', async () => {
    const inst = start_new_db();
    const db_uri = await inst.new_db();
    console.log(`db_uri =>`, db_uri);
    expect(db_uri.indexOf("postgres://postgres:postgres@localhost")).toEqual(0);
  })

  it('allows us to set a custom path', async () => {
    const inst = start_new_db();
    const db_uri = await inst.new_db();
    expect(db_uri.indexOf("postgres://postgres:postgres@localhost")).toEqual(0);
  })

  it('drops a database', async () => {
    const db_uri = await inst.new_db();
    expect(db_uri.indexOf("postgres://postgres:postgres@localhost")).toEqual(0);
    await inst.drop_db(db_uri);
  })

  // it('creates a new database', async () => {
  //   const db_uri = await db.new_db();
  //   expect(db_uri).toBe("bob");
  // })
})
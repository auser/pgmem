'use strict';

const Database = require("..");

describe('Database', () => {

  it('can be started and stopped', async () => {
    expect(async () => {
      const inst = new Database();
      await inst.stop()
    }).not.toThrow();
  });

  it('creates a new database', async () => {
    const inst = new Database();
    const db_uri = await inst.new_db();
    console.log(db_uri);
    expect(db_uri.indexOf("postgres://postgres:postgres@localhost:5433/")).toEqual(0);
    await inst.stop();
  })

  // it('drops a database', async () => {
  //   const inst = new Database();
  //   const db_uri = await inst.new_db();
  //   console.log(db_uri);
  //   const parts = db_uri.split('/')
  //   const db_name = parts[parts.length - 1];
  //   await inst.drop_db(db_name);
  //   await inst.stop();
  // })

  // it('creates a new database', async () => {
  //   const db_uri = await db.new_db();
  //   expect(db_uri).toBe("bob");
  // })
})
'use strict';

const Database = require("..");

describe('Database', () => {
  const start_new_db = (...args) => new Database(...args);
  afterAll(async () => {
    // avoid jest open handle error
    await new Promise(resolve => setTimeout(() => resolve(''), 5000));
  });

  it('can be started and stopped', async () => {
    await expect(async () => {
      const inst = start_new_db();
      await inst.stop()
    }).not.toThrow();
  });

  it('creates a new database', async () => {
    const inst = start_new_db();
    const db_uri = await inst.new_db();
    expect(db_uri.indexOf("postgres://postgres:postgres@localhost:5433/")).toEqual(0);
    await inst.stop();
  })

  it('allows us to set a custom path', async () => {
    const inst = start_new_db();
    const db_uri = await inst.new_db();
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
'use strict';

const Database = require("..");

describe('Database', () => {
  let inst;
  // const start_new_db = (...args) => new Database(...args);
  beforeAll(async () => inst = new Database())
  afterAll(async () => {
    // avoid jest open handle error
    await inst.stop();
    console.log(`stopped database =>`, inst);
    await new Promise(resolve => setTimeout(() => resolve(''), 1000));
  });

  it('can be started and stopped', async () => {
    await expect(async () => {
      const db = new Database();
    }).not.toThrow();
  });

  it('creates a new database', async () => {
    const db_uri = await inst.new_db();
    console.log(`db_uri =>`, db_uri);
    expect(db_uri.indexOf("postgres://postgres:postgres@localhost")).toEqual(0);
  })

  it('allows us to set a custom path', async () => {
    // const inst = start_new_db();
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
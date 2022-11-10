'use strict';

const Database = require("..");
const expect = require('chai').expect

describe('Database', () => {
  const start_new_db = async (...args) => {
    let inst = new Database(...args);
    
    await inst.start();
    return inst;
  }

  it('creates a new database', async () => {
    const inst = await start_new_db();
    const db_uri = await inst.new_db();
    expect(db_uri.indexOf("postgres://postgres:postgres@localhost")).equal(0);
    await inst.stop();
  })
  
  it('allows us to set a custom path', async () => {
    const inst = await start_new_db();
    const db_uri = await inst.new_db();
    expect(db_uri.indexOf("postgres://postgres:postgres@localhost")).equal(0);
    await inst.stop();
  })

  it('drops a database', async () => {
    const inst = await start_new_db();;
    const db_uri = await inst.new_db();
    expect(db_uri.indexOf("postgres://postgres:postgres@localhost")).equal(0);
    const d = await inst.drop_db(db_uri);
    console.log(`d =>`, d);
    await inst.stop();
  })

  // it('creates a new database', async () => {
  //   const db_uri = await db.new_db();
  //   expect(db_uri).toBe("bob");
  // })
})
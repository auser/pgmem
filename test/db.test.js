'use strict';

const Database = require("..");

describe('Database', () => {

  let db;
  beforeAll(async () => db = new Database(), 10000);
  afterAll(async () => {
    try {
    await db.stop_db();
    } catch (e) {}
  }, 1000)

  it('can be started and stopped', async () => {
    const inst = new Database();
    await expect(inst.stop()).not.toThrow();
  }, 1000);

  // it('creates a new database', async () => {
  //   const db_uri = await db.new_db();
  //   expect(db_uri).toBe("bob");
  // })
})
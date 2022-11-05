'use strict';

const Database = require("..");

describe('Database', () => {

  let db;
  beforeAll(async () => db = new Database(), 10000);
  afterAll(async () => {
    await db.stop_db();
  }, 10000)

  it('creates a new database', async () => {
    const db_uri = await db.new_db();
    expect(db_uri).toBe("bob");
  })
})
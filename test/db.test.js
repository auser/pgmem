'use strict';

const Database = require("..");

describe('Database', () => {

  it('can be started and stopped', async () => {
    expect(async () => {
      const inst = new Database();
      await inst.stop()
    }).not.toThrow();
  });

  // it('creates a new database', async () => {
  //   const db_uri = await db.new_db();
  //   expect(db_uri).toBe("bob");
  // })
})
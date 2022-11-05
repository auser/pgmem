'use strict';

const assert = require("assert");

const Database = require("..");

describe('Database', () => {
  it('can be opened', async () => {
    const db = new Database();
    console.log('db =>', db);
    await db.stop_db();
  })
})
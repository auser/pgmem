'use strict';

const Database = require("..");
const expect = require('chai').expect
const { Pool, Client } = require('pg')
const { readFile } = require('fs/promises');
const { join } = require('path');

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
    await inst.drop_db(db_uri);
    await inst.stop();
  })

  it.only('can load sql', async () => {
    const sql = await (await readFile(join(__dirname, "fixtures", "migrations", "migration.sql"))).toString();

    const inst = await start_new_db();;
    const connectionString = await inst.new_db();
    await inst.execute_sql(connectionString, sql);

    const pool = new Pool({
      connectionString,
    })
    console.log('before query');
    const {rows} = await pool.query("SELECT * FROM pg_catalog.pg_tables WHERE schemaname != 'pg_catalog' AND schemaname != 'information_schema';");
    expect(rows.length).equal(2);
    const table_names = rows.map((r) => r.tablename).sort();
    expect(table_names).to.have.members(['AdminUser', 'User']);

    await inst.drop_db(connectionString);
    await inst.stop();
  })

  // it('creates a new database', async () => {
  //   const db_uri = await db.new_db();
  //   expect(db_uri).toBe("bob");
  // })
})
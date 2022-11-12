'use strict';

const Database = require("..");
const expect = require('chai').expect
const { Pool, Client } = require('pg')
const { readFile } = require('fs/promises');
const { join } = require('path');
const { sleep } = require("./utils");

describe('Database', () => {
  let dbs = []
  const start_new_db = async (...args) => {
    let inst = new Database(...args);
    dbs.push(inst);
    
    await inst.start();
    return inst;
  }

  afterEach(async () => {
    await Promise.all(dbs.map(async (db) => db.stop))
  })

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

  it.only('can connect to an external postgres database', async () => {
    let inst = await start_new_db({db_type: "External", uri: "postgres://postgres:postgres@localhost:5432"});
    const db_uri = await inst.new_db();
    console.log(db_uri);
    await inst.drop_db(db_uri)
    await inst.stop();
  });

  it('drops a database', async () => {
    const inst = await start_new_db();;
    const db_uri = await inst.new_db();
    expect(db_uri.indexOf("postgres://postgres:postgres@localhost")).equal(0);
    await inst.drop_db(db_uri);
    await inst.stop();
  })

  it('can load sql', async () => {
    const sql = (await readFile(join(__dirname, "fixtures", "migrations", "migration.sql"))).toString();

    const inst = await start_new_db();;
    const connectionString = await inst.new_db();
    await inst.execute_sql(connectionString, sql);

    const pool = new Pool({
      connectionString,
    })
    const {rows} = await pool.query("SELECT * FROM pg_catalog.pg_tables WHERE schemaname != 'pg_catalog' AND schemaname != 'information_schema';");
    pool.end();
    expect(rows.length).equal(2);
    const table_names = rows.map((r) => r.tablename).sort();
    expect(table_names).to.have.members(['AdminUser', 'User']);

    await inst.drop_db(connectionString);
    await inst.stop();
  })

  it('can spin up a bunch of nodes at the same time', async () => {
    const inst = await start_new_db();;
    const db_uri = await inst.new_db();
    await sleep(100);
    const db_uri2 = await inst.new_db();
    const db_uri3 = await inst.new_db();

    await inst.drop_db(db_uri);
    const db_uri4 = await inst.new_db();
    await sleep(300);
    await inst.drop_db(db_uri2);
    await sleep(100);
    await inst.drop_db(db_uri3);
    await inst.drop_db(db_uri4);

    await inst.stop();
  })

  // it('creates a new database', async () => {
  //   const db_uri = await db.new_db();
  //   expect(db_uri).toBe("bob");
  // })
})
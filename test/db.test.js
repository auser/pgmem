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

  it('can connect to an external postgres database', async () => {
    let inst = await start_new_db({db_type: "External", uri: "postgres://postgres:postgres@localhost:5432"});
    const db_uri = await inst.new_db();
    await inst.drop_db(db_uri)
    await inst.stop();
  }, 2000);

  it('drops a database', async () => {
    const inst = await start_new_db();;
    const db_uri = await inst.new_db();
    expect(db_uri.indexOf("postgres://postgres:postgres@localhost")).equal(0);
    await inst.drop_db(db_uri);
    await inst.stop();
  })

  it('can load sql', async () => {
    const sql = (await readFile(join(__dirname, "fixtures", "migrations", "1_migration.sql"))).toString();

    const inst = await start_new_db();
    const connectionString = await inst.new_db();
    await inst.execute_sql(connectionString, sql);

    await inst.drop_db(connectionString);
    await inst.stop();
  })

  it('can run migrations', async () => {
    const sql = join(__dirname, "fixtures", "migrations");

    let inst = await start_new_db({db_type: "External", uri: "postgres://postgres:postgres@localhost:5432"});
    const connectionString = await inst.new_db();
    console.log('connectionString =>', connectionString);
    await inst.migrate(connectionString, sql);

        const pool = new Pool({
      connectionString,
    })
    const {rows} = await pool.query("SELECT tablename FROM pg_tables;");
    pool.end();
    // expect(rows.length).equal(2);
    const table_names = rows.map((r) => r.tablename).sort();
    expect(table_names.indexOf('AdminUser')).to.be.greaterThanOrEqual(0);
    expect(table_names.indexOf('User')).to.be.greaterThanOrEqual(0);
    // expect(table_names).to.have.members(['AdminUser', 'User']);

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


})
const { promisify } = require("util");

const { init_db, start_db, stop_db, new_db, drop_db } = require("./index.node");

export class Database {
  db: any;
  root_dir: String;

  constructor(root_dir: String) {
    this.root_dir = root_dir ?? ".";
  }

  async init() {
    this.db = await init_db(this.root_dir);
  }

  async start() {
    let db = await this.get_db();
    return start_db.call(db);
  }

  async stop() {
    if (this.db) {
      const res = await stop_db.call(this.db);
      console.log("stopped =>", res);
      delete this.db;
    }
  }

  async new_db(name: string) {
    let db = await this.get_db();
    return new_db.call(db, name);
  }

  async drop_db(uri: string) {
    const parts = uri.split("/") ?? [];
    let db_name = parts[parts.length - 1];
    let db = await this.get_db();
    return drop_db.call(db, uri, db_name);
  }

  async get_db() {
    if (!this.db) {
      await this.init();
    }
    return this.db;
  }
}

module.exports = Database;

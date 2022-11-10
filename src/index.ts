const { promisify } = require("util");

const { init_db, start_db, stop_db, new_db, drop_db } = require("./index.node");

export class Database {
  db: any;
  root_dir: String;
  used = false;

  constructor(root_dir: String) {
    this.root_dir = root_dir ?? ".";
  }

  async start() {
    let db = await this._get_db();
    return db && start_db.call(db);
  }

  async stop() {
    if (this.db) {
      const res = await stop_db.call(this.db);
      this.used = true;
      this.db = undefined;
    }
    return this;
  }

  async new_db(name: string) {
    let db = await this._get_db();
    return db && new_db.call(db, name);
  }

  async drop_db(uri: string) {
    if (this.db) {
      const parts = uri.split("/") ?? [];
      let db_name = parts[parts.length - 1];
      await drop_db.call(this.db, uri, db_name);
    }
  }

  async _get_db() {
    if (!this.db) {
      this.db = init_db(this.root_dir);
    }
    return this.db;
  }
}

module.exports = Database;

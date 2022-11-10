const { promisify } = require("util");
import * as url from "node:url";

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
      const url_obj = new URL(uri);
      const db_name = url_obj.pathname.split("/")[1];
      const parsed_uri = url.format(url_obj, { search: false });
      await drop_db.call(this.db, parsed_uri, db_name);
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

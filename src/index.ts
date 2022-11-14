import * as url from "node:url";

const {
  init_db,
  start_db,
  stop_db,
  new_db,
  drop_db,
  execute_sql,
  db_migration,
} = require("./index.node");

export enum DB_TYPE {
  EXTERNAL = "External",
  EMBEDDED = "Embedded",
}

export type DatabaseOptions = {
  db_type: DB_TYPE;
  uri: string;
  root_path?: string;
  username?: string;
  password?: string;
  persistent?: boolean;
  port?: number;
  timeout?: number;
  host?: string;
};

const default_options: DatabaseOptions = {
  db_type: DB_TYPE.EMBEDDED,
  uri: "127.0.0.1",
  username: "postgres",
  password: "postgres",
  timeout: 1000,
  host: "127.0.0.1",
};
export class Database {
  db: any;
  options: DatabaseOptions;
  used = false;

  constructor(options: DatabaseOptions) {
    this.options = { ...default_options, ...options };
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

  async new_db(name?: string) {
    let db = await this._get_db();
    return db && new_db.call(db, name);
  }

  async run_migrations(uri: string, migrations_dir: string) {
    let db = await this._get_db();
    let db_name = this._get_db_name_from_uri(uri);
    console.log(
      "AKSDJFKASJDFKASJDKFAJSDKFAJSDKFJASDKFJ",
      db_name,
      migrations_dir
    );

    return db && db_migration.call(db, db_name, migrations_dir);
  }

  async execute_sql(uri: string, sql: string) {
    let db = await this._get_db();
    return db && execute_sql.call(db, sql);
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
      this.db = init_db(this.options);
    }
    return this.db;
  }

  _get_db_name_from_uri(uri: string) {
    const url_obj = new URL(uri);
    const db_name = url_obj.pathname.split("/")[1];
    return db_name;
  }
}

export default Database;

const { promisify } = require("util");

const { start_db, stop_db } = require("./index.node");

class Database {
  constructor() {
    this.db = start_db();
    console.log('this.db =>', this.db);
  }

  stop_db() {
    stop_db.call(this.db)
  }
}

module.exports = Database

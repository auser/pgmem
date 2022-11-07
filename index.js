const { promisify } = require("util");

const { init, start, stop, new_db } = require("./index.node");

class Database {
  constructor() {
    this.db = init();
    console.log('this.db =>', this.db);
  }

  async start() {
    start.call(this.db)
  }

  async stop() {
    stop.call(this.db)
  }
}

module.exports = Database

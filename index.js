const { promisify } = require("util");

const { init, start, stop, new_db } = require("./index.node");

class Database {
  constructor() {
    this.db = init();
  }

  async start() {
    return start.call(this.db)
  }

  async stop() {
    return stop.call(this.db)
  }

  async new_db(name) {
    return new_db.call(this.db, name)
  }
}

module.exports = Database

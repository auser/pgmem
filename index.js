const { promisify } = require("util");

const { init, start, stop, new_db, drop_db } = require("./index.node");

class Database {
  constructor(root_dir) {
    root_dir = root_dir ?? ".";
    this.db = init(root_dir);
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

  async drop_db(name) {
    return drop_db.call(this.db, name)
  }
}

module.exports = Database

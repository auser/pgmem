# pmem

Want to run some database driven tests? So did we... but testing against a database between isolated tests is hard. That's where pmem comes into play.

## Usage

First create an instance of the database system:

```typescript
const db = new Database()
await db.start();
```

When you want to create a new database, run `new_db()`:

```typescript
const new_uri = await db.new_db();
```

When you're done with the database, kill it:

```typescript
await db.stop(new_uri);
```

When you're done, clean up!

```typescript
await db.cleanup();
```

## TODO

- [ ] Change database creation into it's own instance
- [ ] Explore using vectors of DBLocks

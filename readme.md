# BANTER Workspace

## Packages

### Shared

shared bussiness logic and app utilities

- `http-core`, shared abstraction over http
- `types`, bussiness logic structs
- `sql`, all sql queries
- `auth`, authentication

### Router

bridge runtime and bussiness logic

- `api`, router for json response

### Entry Points

the root package is the entry points, it start
tokio runtime, tcp listener, and db pool

## Usage

### Prerequisite

- `.env` containing DATABASE_URL for postgres

```bash
echo 'DATABASE_URL=postgres' > .env
```

### Server

root package is the entry point:

```bash
# Debug
cargo run
# Release
cargo run --release
```

### Database

migrations are store in `migrations/`

create/drop the database at DATABASE_URL

```bash
sqlx database create
sqlx database drop
```

run/revert migration

```bash
sqlx migrate run
sqlx migrate revert
```


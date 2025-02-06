# ontos

Copenheimer who?

Eventually we'll put something here but for the meantime we're the best scanner out there

## setup

```bash
git clone git@github.com:kolatra/scanner.git
mv .env.example .env
cargo install sea-orm-cli

# start the database and populate
just postgres
just migrate

# without just
docker compose -f docker-compose.db.yml up -d
sea-orm-cli migrate up
```

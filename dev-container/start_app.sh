#! /bin/bash

set -e
echo RUNNING MIGRATIONS
cd /app/db && diesel migration run && cd ..

echo CHECKING DATABASE
psql --version

TABLE_COUNT=$(psql -h postgres -p 5432 -U hive-dev -d hive-local -c "select count(*) from users" -At)
if [ $TABLE_COUNT -eq 0 ]; then
  echo POPULATING DATABASE
  echo ${psql --version}
  psql -h postgres -p 5432 -U hive-dev -d hive-local -f /app/dev-container/db_files/hive_dump.sql
fi

echo STARTING APP
rustup target add wasm32-unknown-unknown
cargo leptos watch --hot-reload

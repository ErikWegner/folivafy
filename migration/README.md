# Running Migrator CLI

- Generate a new migration file
    ```sh
    cargo run -p migration -- generate MIGRATION_NAME
    ```
- Apply all pending migrations
    ```sh
    cargo run
    ```
    ```sh
    cargo run -p migration -- up
    ```
- Apply first 10 pending migrations
    ```sh
    cargo run -p migration -- up -n 10
    ```
- Rollback last applied migrations
    ```sh
    cargo run -p migration -- down
    ```
- Rollback last 10 applied migrations
    ```sh
    cargo run -p migration -- down -n 10
    ```
- Drop all tables from the database, then reapply all migrations
    ```sh
    cargo run -p migration -- fresh
    ```
- Rollback all applied migrations, then reapply all migrations
    ```sh
    cargo run -p migration -- refresh
    ```
- Rollback all applied migrations
    ```sh
    cargo run -p migration -- reset
    ```
- Check the status of all migrations
    ```sh
    cargo run -p migration -- status
    ```

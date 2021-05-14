# Group Chat

WIP [Discuss](https://github.com/CympleTech/esse/discussions/5)


## Database prepare
``` shell
$ export DATABASE_URL=postgres://postgres@localhost/my_database
$ cargo install sqlx-cli --no-default-features --features postgres
$ sqlx database create
$ sqlx migrate run
```
[more details about sqlx](https://github.com/launchbadge/sqlx/tree/master/sqlx-cli)


## Running
``` shell
$ cargo run

```

# Group Chat

WIP

## Running
`export DATABASE_URL=postgres://postgres@localhost/my_database`

## Database prepare
``` shell
$ cargo install sqlx-cli --no-default-features --features postgres
$ sqlx database create
$ sqlx migrate run
```
[more details about sqlx](https://github.com/launchbadge/sqlx/tree/master/sqlx-cli)

[Discuss](https://github.com/CympleTech/esse/discussions/5)

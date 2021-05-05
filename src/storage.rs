use postgres::{Client, NoTls};
use tdn::types::primitive::{new_io_error, Result};

pub(crate) fn connect_database() -> Result<Client> {
    Client::connect("host=localhost user=user password=password", NoTls).map_err(|_e| {
        println!("{:?}", _e);
        new_io_error("postgres connect failure.")
    })
}

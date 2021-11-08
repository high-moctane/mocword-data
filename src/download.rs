use std::io;
use std::thread;
use std::time;

use diesel::Connection;
use diesel::MysqlConnection;

use crate::embedded_migrations;

pub fn run() {
    thread::sleep(time::Duration::from_secs(20));
    let conn = new_conn();
    migrate(&conn);
}

fn mariadb_dsn() -> String {
    format!("mysql://moctane:pw@mariadb:3306/mocword")
}

fn new_conn() -> MysqlConnection {
    MysqlConnection::establish(&mariadb_dsn()).unwrap()
}

fn migrate(conn: &MysqlConnection) {
    embedded_migrations::run_with_output(conn, &mut io::stdout()).unwrap();
}

extern crate mariadb_proxy;

extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate tokio;

use std::env;
use std::collections::HashMap;
use mariadb_proxy::packet::{Packet, PacketType};
use mariadb_proxy::packet_handler::{PacketHandler};

struct CounterHandler {
  count_map: HashMap<String, u64>,
}

// Just forward the packet
impl PacketHandler for CounterHandler {

  fn handle_request(&mut self, p: &Packet) -> Packet {
    // Print out the packet
    //debug!("[{}]", String::from_utf8_lossy(&p.bytes));

    match p.packet_type() {
      Ok(PacketType::ComQuery) => {
        let payload = &p.bytes[5..];
        let sql = String::from_utf8(payload.to_vec()).expect("Invalid UTF-8");
        info!("SQL: {}", sql);
        let tokens: Vec<&str> = sql.split(' ').collect();
        let command = tokens[0].to_lowercase();
        let count = self.count_map.entry(command).or_insert(0);
        *count += 1;
        println!("{:?}", self.count_map);
        //info!("{}", tokens);
      },
      _ => {
        debug!("{:?} packet", p.packet_type())
      },
    }

    Packet { bytes: p.bytes.clone() }
  }

  fn handle_response(&mut self, p: &Packet) -> Packet {
    Packet { bytes: p.bytes.clone() }
  }

}

#[tokio::main]
async fn main() {
  env_logger::init();

  info!("Counter MariaDB proxy... ");

  // determine address for the proxy to bind to
  let bind_addr = env::args().nth(1).unwrap_or("0.0.0.0:3306".to_string());
  // determine address of the MariaDB instance we are proxying for
  let db_addr = env::args().nth(2).unwrap_or("mariadb:3306".to_string());

  let mut server = mariadb_proxy::server::Server::new(bind_addr.clone(), db_addr.clone()).await;
  info!("Proxy listening on: {}", bind_addr);
  server.run(CounterHandler { count_map: HashMap::new() }).await;
}


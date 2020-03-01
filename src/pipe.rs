use crate::{
    packet::{DatabaseType, Packet},
    packet_handler::{Direction, PacketHandler},
};
use std::{
    io::{Error, ErrorKind},
    sync::{Arc, Mutex},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt, Result};

pub struct Pipe<T: AsyncReadExt, U: AsyncWriteExt> {
    name: String,
    db_type: DatabaseType,
    packet_handler: Arc<Mutex<dyn PacketHandler + Send>>,
    direction: Direction,
    source: T,
    sink: U,
}

impl<T: AsyncReadExt + Unpin, U: AsyncWriteExt + Unpin> Pipe<T, U> {
    pub fn new(
        name: String,
        db_type: DatabaseType,
        packet_handler: Arc<Mutex<dyn PacketHandler + Send>>,
        direction: Direction,
        reader: T,
        writer: U,
    ) -> Pipe<T, U> {
        Pipe {
            name,
            db_type,
            packet_handler,
            direction,
            source: reader,
            sink: writer,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        trace!("[{}]: Running {:?} pipe loop...", self.name, self.direction);
        //let source = Arc::get_mut(&mut self.source).unwrap();
        //let sink = Arc::get_mut(&mut self.sink).unwrap();
        let mut read_buf: Vec<u8> = vec![0_u8; 4096];
        let mut packet_buf: Vec<u8> = Vec::with_capacity(4096);
        let mut write_buf: Vec<u8> = Vec::with_capacity(4096);

        loop {
            // Read from the source to read_buf, append to packet_buf
            let n = self.source.read(&mut read_buf[..]).await?;
            trace!(
                "[{}:{:?}]: Read {} bytes from client",
                self.name,
                self.direction,
                n
            );
            if n == 0 {
                let e = Error::new(
                    ErrorKind::Other,
                    format!(
                        "[{}:{:?}]: Read {} bytes, closing pipe.",
                        self.name, self.direction, n
                    ),
                );
                warn!("{}", e.to_string());
                return Err(e);
            }
            trace!(
                "[{}:{:?}]: {} bytes read from source",
                self.name,
                self.direction,
                n
            );
            packet_buf.extend_from_slice(&read_buf[0..n]);

            // Process all packets in packet_buf, put into write_buf
            while let Some(packet) = get_packet(self.db_type, &mut packet_buf) {
                debug!("[{}:{:?}]: Processing packet", self.name, self.direction);
                {
                    // Scope for self.packet_handler Mutex
                    let mut h = self.packet_handler.lock().unwrap();
                    let transformed_packet = match self.direction {
                        Direction::Forward => h.handle_request(&packet),
                        Direction::Backward => h.handle_response(&packet),
                    };
                    write_buf.extend_from_slice(&transformed_packet.bytes);
                }
            }

            // Write all to sink
            let n = self.sink.write(&write_buf[..]).await?;
            let _: Vec<u8> = write_buf.drain(0..n).collect();
            trace!(
                "[{}:{:?}]: {} bytes written to sink",
                self.name,
                self.direction,
                n
            );
        } // end loop
    }
}

fn get_packet(db_type: DatabaseType, packet_buf: &mut Vec<u8>) -> Option<Packet> {
    match db_type {
        DatabaseType::MariaDB => {
            // Check for header
            if packet_buf.len() > 3 {
                let l: usize = (((packet_buf[2] as u32) << 16)
                    | ((packet_buf[1] as u32) << 8)
                    | packet_buf[0] as u32) as usize;
                let s = 4 + l;
                // Check for entire packet size
                if packet_buf.len() >= s {
                    let p = Packet::new(DatabaseType::MariaDB, packet_buf.drain(0..s).collect());
                    Some(p)
                } else {
                    None
                }
            } else {
                None
            }
        }
        DatabaseType::PostgresSQL => {
            //TODO
            None
        }
    }
}

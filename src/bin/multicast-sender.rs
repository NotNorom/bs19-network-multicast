use std::{
    io::{ErrorKind::TimedOut, Read},
    num::NonZeroU16,
    time::Duration,
};

use bs19_network::{
    ADDRESS, ReceiveError, create_socket, join_multicast, universe_to_ipv4_multicast_addr,
};

fn main() {
    let mut socket = create_socket(ADDRESS).expect("socket creation");
    let multicast_addr = universe_to_ipv4_multicast_addr(NonZeroU16::new(4).unwrap());

    join_multicast(&socket, &multicast_addr, None).expect("join multicast address");

    socket
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("set read timeout");

    let mut buffer = [0u8; 1024];

    let error = loop {
        let read_res = socket.read(&mut buffer).map_err(ReceiveError::from);

        let bytes_read = match read_res.map_err(ReceiveError::from) {
            Ok(read_bytes) => read_bytes,
            Err(err) => match err {
                ReceiveError::Io(ref io_error) => match io_error.kind() {
                    TimedOut => continue,
                    _ => break err,
                },
                _ => break err,
            },
        };

        println!(">> {:?}", &buffer[0..bytes_read]);
    };

    eprintln!("Error: {error:#?}");

    join_multicast(&socket, &multicast_addr, None).unwrap();
}

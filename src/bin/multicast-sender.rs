use std::{
    io::{BufRead, ErrorKind::TimedOut, Read, Write},
    time::Duration,
};

use bs19_network::{ADDRESS, Cli, ReceiveError, create_socket, universe_to_ipv4_multicast_addr};
use clap::Parser;

const BUFFER_SIZE: usize = 1024;

fn main() {
    let cli = Cli::parse();

    if cli.universes.is_empty() {
        println!("Specify at least one universe");
        return;
    }

    let socket = create_socket(ADDRESS).expect("socket creation");

    let mut multicast_addresses = Vec::with_capacity(4);
    for universe in &cli.universes {
        multicast_addresses.push(universe_to_ipv4_multicast_addr(*universe));
    }

    socket
        .set_write_timeout(Some(Duration::from_secs(1)))
        .expect("set write timeout");

    let mut buffer = String::with_capacity(BUFFER_SIZE);
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();

    println!(
        "sending to addresses {:?}",
        multicast_addresses
            .iter()
            .map(|addr| addr.as_socket_ipv4().unwrap())
            .collect::<Vec<_>>()
    );

    let error = loop {
        print!(">> ");
        let _ = stdout.flush();

        buffer.clear();

        let read_res = stdin
            .by_ref()
            .take(BUFFER_SIZE as u64)
            .read_line(&mut buffer);

        let bytes_read = match read_res.map_err(ReceiveError::from) {
            Ok(read_bytes) => read_bytes,
            Err(err) => match err {
                ReceiveError::Io(ref io_error) => match io_error.kind() {
                    TimedOut => {
                        continue;
                    }
                    _ => break err,
                },
                _ => break err,
            },
        };

        let errors = multicast_addresses
            .iter()
            .map(|multicast_addr| socket.send_to(&buffer.as_bytes()[0..bytes_read], multicast_addr))
            .filter_map(|res| res.err())
            .collect::<Vec<_>>();

        if !errors.is_empty() {
            eprintln!("Error sending to multicast addresses: {:?}", errors);
        } else {
            println!("Sent!");
        }
    };

    eprintln!("Error: {error:#?}");
}

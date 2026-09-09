use std::{
    io::ErrorKind::{TimedOut, WouldBlock},
    mem::MaybeUninit,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use bs19_network::{
    ADDRESS, Cli, ReceiveError, create_socket, initialized_bytes, join_multicast, leave_multicast,
    print_buffer, universe_to_ipv4_multicast_addr,
};
use clap::Parser;

fn main() {
    let running = Arc::new(AtomicBool::new(true));

    let cli = Cli::parse();

    let socket = create_socket(ADDRESS).expect("socket creation");

    let mut multicast_addresses = Vec::with_capacity(4);
    for universe in &cli.universes {
        multicast_addresses.push(universe_to_ipv4_multicast_addr(*universe));
    }

    for multicast_addr in &multicast_addresses {
        join_multicast(&socket, multicast_addr, None).expect("join multicast address");
    }

    socket
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("set read timeout");

    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let mut buffer = [MaybeUninit::new(0u8); 1024];

    println!(
        "listening on addresses {:?}",
        &multicast_addresses
            .iter()
            .map(|addr| addr.as_socket_ipv4().unwrap())
            .collect::<Vec<_>>()
    );

    let error = loop {
        if !running.load(Ordering::SeqCst) {
            break ReceiveError::CtrlC;
        }

        buffer.fill(MaybeUninit::zeroed());

        let read_res = socket.recv_from(&mut buffer).map_err(ReceiveError::from);
        println!("{read_res:?}");

        let (bytes_read, sender_addr) = match read_res.map_err(ReceiveError::from) {
            Ok(read_bytes) => read_bytes,
            Err(err) => match err {
                ReceiveError::Io(ref io_error) => match io_error.kind() {
                    TimedOut => {
                        continue;
                    }
                    WouldBlock => {
                        continue;
                    }
                    _ => break err,
                },
                _ => break err,
            },
        };

        let nicer_buffer = initialized_bytes(&buffer, bytes_read);

        println!(">> {:?}:", sender_addr.as_socket_ipv4().unwrap());
        print_buffer(nicer_buffer);
    };

    eprintln!("Error: {error:#?}");

    for multicast_addr in &multicast_addresses {
        leave_multicast(&socket, multicast_addr, None).expect("leaving multicast address")
    }
}

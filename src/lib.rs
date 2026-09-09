use std::{
    mem::MaybeUninit,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4},
    num::NonZeroU16,
};

use socket2::{Domain, SockAddr, Socket, Type};

mod error;
pub use error::ReceiveError;

mod cli;
pub use cli::Cli;

pub const PORT: u16 = 5568; // default sacn port = 5568
pub const ADDRESS: SocketAddr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, PORT));

/// Creates a new Socket2 socket bound to the given address.
///
/// Returns the created socket.
///
/// Arguments:
/// addr: The address that the newly created socket should bind to.
///
/// # Errors
/// Will return an error if the socket cannot be created, see (Socket::new)[fn.new.Socket].
///
/// Will return an error if the socket cannot be bound to the given address, see (bind)[fn.bind.Socket2].
pub fn create_socket(addr: SocketAddr) -> Result<Socket, ReceiveError> {
    let socket = Socket::new(Domain::for_address(addr), Type::DGRAM, None)?;

    // Multiple different processes might want to listen to the sACN stream so therefore need to allow re-using the ACN port.
    #[cfg(not(target_os = "windows"))]
    {
        socket.set_reuse_port(true)?;
    }

    socket.set_reuse_address(true)?;
    socket.set_multicast_loop_v4(true)?;
    socket.set_multicast_all_v4(true)?;

    let ip = match addr.ip() {
        // after many many MANY hours of testing and research I figured out:
        // for receiving multicast you need to bind to 0.0.0.0, or ::
        // If you pass a regular ip address here because you thought: "hey, I wanna listen to
        // the interface that this IP belongs to" then don't. It will not receive any packets
        // and you will be left wondering why the hell it doesn't.
        IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
    };

    let socket_addr = SocketAddr::new(ip, PORT);

    socket.bind(&socket_addr.into())?;
    Ok(socket)
}

/// Joins the multicast group with the given address using the given socket.
///
/// Arguments:
/// socket: The socket to join to the multicast group.
/// addr:   The address of the multicast group to join.
///
/// # Errors
/// Will return an error if the given socket cannot be joined to the given multicast group address.
///     See join_multicast_v4[fn.join_multicast_v4.Socket] and join_multicast_v6[fn.join_multicast_v6.Socket]
///
/// Will return an IpVersionError if addr and interface_addr are not the same IP version.
pub fn join_multicast(
    socket: &Socket,
    multicast_addr: &SockAddr,
    interface_addr: Option<IpAddr>,
) -> Result<(), ReceiveError> {
    match multicast_addr.as_socket().unwrap() {
        SocketAddr::V4(addr) => {
            match interface_addr {
                Some(IpAddr::V4(ref interface_v4)) => {
                    socket.join_multicast_v4(addr.ip(), interface_v4)?;
                }
                Some(IpAddr::V6(_)) => {
                    Err(ReceiveError::IpVersionError(
                        "Multicast address and interface_addr not same IP version".to_string(),
                    ))?;
                }
                None => socket.join_multicast_v4(addr.ip(), &Ipv4Addr::UNSPECIFIED)?,
            };
        }
        SocketAddr::V6(addr) => match interface_addr {
            Some(IpAddr::V4(_)) => {
                Err(ReceiveError::IpVersionError(
                    "Multicast address and interface_addr not same IP version".to_string(),
                ))?;
            }
            Some(IpAddr::V6(_)) => socket.join_multicast_v6(addr.ip(), 0)?,
            None => socket.join_multicast_v6(addr.ip(), 0)?,
        },
    }

    Ok(())
}

/// Leaves the multicast group with the given address using the given socket.
///
/// Arguments:
/// socket: The socket to leave the multicast group.
/// addr:   The address of the multicast group to leave.
///
/// # Errors
/// Will return an error if the given socket cannot leave the given multicast group address.
///     See leave_multicast_v4[fn.leave_multicast_v4.Socket] and leave_multicast_v6[fn.leave_multicast_v6.Socket]
///
/// Will return an IpVersionError if addr and interface_addr are not the same IP version.
pub fn leave_multicast(
    socket: &Socket,
    addr: &SockAddr,
    interface_addr: Option<IpAddr>,
) -> Result<(), ReceiveError> {
    match addr.as_socket().unwrap() {
        SocketAddr::V4(addr) => match interface_addr {
            Some(IpAddr::V4(ref interface_v4)) => {
                socket.leave_multicast_v4(addr.ip(), interface_v4)?;
            }
            Some(IpAddr::V6(ref _interface_v6)) => {
                Err(ReceiveError::IpVersionError(
                    "Multicast address and interface_addr not same IP version".to_string(),
                ))?;
            }
            None => {
                socket.leave_multicast_v4(addr.ip(), &Ipv4Addr::UNSPECIFIED)?;
            }
        },
        SocketAddr::V6(addr) => match interface_addr {
            Some(IpAddr::V4(_)) => {
                Err(ReceiveError::IpVersionError(
                    "Multicast address and interface_addr not same IP version".to_string(),
                ))?;
            }
            Some(IpAddr::V6(_)) => socket.leave_multicast_v6(addr.ip(), 0)?,
            None => socket.leave_multicast_v6(addr.ip(), 0)?,
        },
    };

    Ok(())
}

/// Converts the given ANSI E1.31-2018 universe into an Ipv4 multicast address
///
/// Conversion done as specified in section 9.3.1 of ANSI E1.31-2018
///
/// Returns the multicast address.
pub fn universe_to_ipv4_multicast_addr(universe: NonZeroU16) -> SockAddr {
    let high_byte: u8 = ((universe.get() >> 8) & 0xff) as u8;
    let low_byte: u8 = (universe.get() & 0xff) as u8;

    // As per ANSI E1.31-2018 Section 9.3.1 Table 9-10.
    SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(239, 255, high_byte, low_byte)),
        PORT,
    )
    .into()
}

pub fn initialized_bytes(buf: &[MaybeUninit<u8>], len: usize) -> &[u8] {
    debug_assert!(len <= buf.len());

    unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), len) }
}

pub fn print_buffer(buffer: &[u8]) {
    for chunk in buffer.chunks(32) {
        let chunk_as_str = { String::from_utf8_lossy(chunk) };

        // let chunk_as_str = String::from_utf8(
        //     chunk
        //         .as_ref()
        //         .iter()
        //         // .filter(|b| {
        //         //     b.is_ascii_alphanumeric()
        //         //         || b.is_ascii_graphic()
        //         //         || b.is_ascii_hexdigit()
        //         //         || b.is_ascii_punctuation()
        //         //         || **b == b' '
        //         // })
        //         .map(|b| escape_default(*b))
        //         .flatten()
        //         .collect(),
        // )
        // .unwrap();
        for byte in chunk {
            print!("{byte:0>3} ");
        }
        println!(" {chunk_as_str}");
    }
}

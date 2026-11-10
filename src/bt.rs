//! Bluetooth RFCOMM (serial port profile) transport, so the desktop app can reach
//! the player over Bluetooth the way the Android app does when Wi-Fi is unavailable.

use std::io::{self, Read, Write};

/// RFCOMM channel the player registers its SonixLink service on.
pub const RFCOMM_CHANNEL: u8 = 22;

/// "AA:BB:CC:DD:EE:FF" (or 12 hex digits) as a Bluetooth address.
pub fn parse_address(text: &str) -> Option<u64> {
    let hex: String = text.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hex.len() != 12 {
        return None;
    }
    u64::from_str_radix(&hex, 16).ok()
}

#[cfg(windows)]
mod platform {
    use std::io::{self, Read, Write};
    use std::ptr;
    use std::sync::Once;

    type Socket = isize;

    #[repr(C)]
    struct SockAddrBth {
        family: u16,
        address: u64,
        service_class_id: [u8; 16],
        port: u32,
    }

    const AF_BTH: i32 = 32;
    const BTHPROTO_RFCOMM: i32 = 3;
    const SOCK_STREAM: i32 = 1;
    const SOL_SOCKET: i32 = 0xffff;
    const SO_RCVTIMEO: i32 = 0x1006;
    const SO_SNDTIMEO: i32 = 0x1005;
    const INVALID_SOCKET: Socket = -1;

    #[link(name = "ws2_32")]
    extern "system" {
        fn WSAStartup(version: u16, data: *mut u8) -> i32;
        fn WSASocketW(af: i32, kind: i32, protocol: i32, info: *mut u8, group: u32, flags: u32) -> Socket;
        fn connect(socket: Socket, name: *const u8, len: i32) -> i32;
        fn recv(socket: Socket, buf: *mut u8, len: i32, flags: i32) -> i32;
        fn send(socket: Socket, buf: *const u8, len: i32, flags: i32) -> i32;
        fn setsockopt(socket: Socket, level: i32, name: i32, value: *const u8, len: i32) -> i32;
        fn closesocket(socket: Socket) -> i32;
        fn WSAGetLastError() -> i32;
    }

    fn last_error() -> io::Error {
        io::Error::from_raw_os_error(unsafe { WSAGetLastError() })
    }

    pub struct Rfcomm {
        socket: Socket,
    }

    impl Rfcomm {
        pub fn connect(address: u64, channel: u8, timeout_ms: u32) -> io::Result<Self> {
            // WSAStartup is reference counted by Winsock, but one call is enough here
            static START: Once = Once::new();
            START.call_once(|| unsafe {
                let mut data = [0u64; 64];
                WSAStartup(0x0202, data.as_mut_ptr() as *mut u8);
            });

            let socket = unsafe { WSASocketW(AF_BTH, SOCK_STREAM, BTHPROTO_RFCOMM, ptr::null_mut(), 0, 0) };
            if socket == INVALID_SOCKET {
                return Err(last_error());
            }

            unsafe {
                let timeout = timeout_ms;
                setsockopt(socket, SOL_SOCKET, SO_RCVTIMEO, &timeout as *const u32 as *const u8, 4);
                setsockopt(socket, SOL_SOCKET, SO_SNDTIMEO, &timeout as *const u32 as *const u8, 4);
            }

            let addr = SockAddrBth {
                family: AF_BTH as u16,
                address,
                service_class_id: [0; 16],
                port: channel as u32,
            };
            let rc = unsafe {
                connect(
                    socket,
                    &addr as *const SockAddrBth as *const u8,
                    std::mem::size_of::<SockAddrBth>() as i32,
                )
            };
            if rc != 0 {
                let err = last_error();
                unsafe { closesocket(socket) };
                return Err(err);
            }

            Ok(Self { socket })
        }
    }

    impl Read for Rfcomm {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = unsafe { recv(self.socket, buf.as_mut_ptr(), buf.len() as i32, 0) };
            if n < 0 {
                Err(last_error())
            } else {
                Ok(n as usize)
            }
        }
    }

    impl Write for Rfcomm {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let n = unsafe { send(self.socket, buf.as_ptr(), buf.len() as i32, 0) };
            if n < 0 {
                Err(last_error())
            } else {
                Ok(n as usize)
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Drop for Rfcomm {
        fn drop(&mut self) {
            unsafe { closesocket(self.socket) };
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use std::io;

    pub struct Rfcomm;

    impl Rfcomm {
        pub fn connect(_address: u64, _channel: u8, _timeout_ms: u32) -> io::Result<Self> {
            Err(io::Error::new(io::ErrorKind::Unsupported, "Bluetooth is Windows only"))
        }
    }

    impl std::io::Read for Rfcomm {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(std::io::ErrorKind::Unsupported.into())
        }
    }

    impl std::io::Write for Rfcomm {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(std::io::ErrorKind::Unsupported.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}

pub use platform::Rfcomm;

/// Read and write, whichever link the client is using.
pub enum Link {
    Tcp(std::net::TcpStream),
    Bluetooth(Rfcomm),
}

impl Read for Link {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Link::Tcp(s) => s.read(buf),
            Link::Bluetooth(s) => s.read(buf),
        }
    }
}

impl Write for Link {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Link::Tcp(s) => s.write(buf),
            Link::Bluetooth(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Link::Tcp(s) => s.flush(),
            Link::Bluetooth(s) => s.flush(),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_mac_addresses() {
        assert_eq!(super::parse_address("AA:BB:CC:DD:EE:FF"), Some(0xAABBCCDDEEFF));
        assert_eq!(super::parse_address("aabbccddeeff"), Some(0xAABBCCDDEEFF));
        assert_eq!(super::parse_address("nope"), None);
        assert_eq!(super::parse_address("AA:BB"), None);
    }
}

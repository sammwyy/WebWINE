use super::{Handled, WinApiRegistry};

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[

        // Winsock (ws2_32): present so apps like putty init networking and reach
        // their UI. Socket ops fail (no real network); init + byte-swap work.
        ("ws2_32.dll", "WSAStartup", |c| {
            let d = c.arg(1);
            if d != 0 {
                let _ = c.memory.write_u16(d, 0x0202);
                let _ = c.memory.write_u16(d + 2, 0x0202);
            }
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("ws2_32.dll", "WSACleanup", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }),
        ("ws2_32.dll", "WSAGetLastError", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }),
        ("ws2_32.dll", "WSASetLastError", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("ws2_32.dll", "WSACreateEvent", |c| {
            c.ret_stdcall(0, 0);
            Handled::Ok
        }),
        ("ws2_32.dll", "WSACloseEvent", |c| {
            c.ret_stdcall(1, 1);
            Handled::Ok
        }),
        ("ws2_32.dll", "WSAAsyncSelect", |c| {
            c.ret_stdcall(0, 4);
            Handled::Ok
        }),
        ("ws2_32.dll", "WSAEventSelect", |c| {
            c.ret_stdcall(0, 3);
            Handled::Ok
        }),
        ("ws2_32.dll", "WSAIoctl", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 9);
            Handled::Ok
        }),
        ("ws2_32.dll", "WSAAddressToStringA", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 5);
            Handled::Ok
        }),
        ("ws2_32.dll", "WSAStringToAddressA", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 5);
            Handled::Ok
        }),
        ("ws2_32.dll", "socket", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 3);
            Handled::Ok
        }), // INVALID_SOCKET
        ("ws2_32.dll", "WSASocketA", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 6);
            Handled::Ok
        }),
        ("ws2_32.dll", "closesocket", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("ws2_32.dll", "connect", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 3);
            Handled::Ok
        }),
        ("ws2_32.dll", "bind", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 3);
            Handled::Ok
        }),
        ("ws2_32.dll", "listen", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 2);
            Handled::Ok
        }),
        ("ws2_32.dll", "accept", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 3);
            Handled::Ok
        }),
        ("ws2_32.dll", "send", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 4);
            Handled::Ok
        }),
        ("ws2_32.dll", "recv", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 4);
            Handled::Ok
        }),
        ("ws2_32.dll", "sendto", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 6);
            Handled::Ok
        }),
        ("ws2_32.dll", "recvfrom", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 6);
            Handled::Ok
        }),
        ("ws2_32.dll", "shutdown", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("ws2_32.dll", "select", |c| {
            c.ret_stdcall(0, 5);
            Handled::Ok
        }),
        ("ws2_32.dll", "ioctlsocket", |c| {
            c.ret_stdcall(0, 3);
            Handled::Ok
        }),
        ("ws2_32.dll", "getsockname", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 3);
            Handled::Ok
        }),
        ("ws2_32.dll", "getpeername", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 3);
            Handled::Ok
        }),
        ("ws2_32.dll", "getsockopt", |c| {
            c.ret_stdcall(0, 5);
            Handled::Ok
        }),
        ("ws2_32.dll", "setsockopt", |c| {
            c.ret_stdcall(0, 5);
            Handled::Ok
        }),
        ("ws2_32.dll", "gethostname", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("ws2_32.dll", "gethostbyname", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("ws2_32.dll", "getaddrinfo", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 4);
            Handled::Ok
        }),
        ("ws2_32.dll", "freeaddrinfo", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("ws2_32.dll", "getnameinfo", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 7);
            Handled::Ok
        }),
        ("ws2_32.dll", "inet_addr", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 1);
            Handled::Ok
        }),
        ("ws2_32.dll", "inet_ntoa", |c| {
            c.ret_stdcall(0, 1);
            Handled::Ok
        }),
        ("ws2_32.dll", "htons", |c| {
            let v = c.arg(0) as u16;
            c.ret_stdcall(v.swap_bytes() as u32, 1);
            Handled::Ok
        }),
        ("ws2_32.dll", "ntohs", |c| {
            let v = c.arg(0) as u16;
            c.ret_stdcall(v.swap_bytes() as u32, 1);
            Handled::Ok
        }),
        ("ws2_32.dll", "htonl", |c| {
            let v = c.arg(0);
            c.ret_stdcall(v.swap_bytes(), 1);
            Handled::Ok
        }),
        ("ws2_32.dll", "ntohl", |c| {
            let v = c.arg(0);
            c.ret_stdcall(v.swap_bytes(), 1);
            Handled::Ok
        }),
        ("ws2_32.dll", "__WSAFDIsSet", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("ws2_32.dll", "WSAWaitForMultipleEvents", |c| {
            c.ret_stdcall(0xFFFF_FFFF, 5);
            Handled::Ok
        }),
        ("ws2_32.dll", "WSAEnumNetworkEvents", |c| {
            c.ret_stdcall(0, 3);
            Handled::Ok
        }),
        ("ws2_32.dll", "WSAGetOverlappedResult", |c| {
            c.ret_stdcall(0, 5);
            Handled::Ok
        }),
        ("ws2_32.dll", "getservbyname", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("ws2_32.dll", "getservbyport", |c| {
            c.ret_stdcall(0, 2);
            Handled::Ok
        }),
        ("ws2_32.dll", "inet_ntop", |c| {
            c.ret_stdcall(0, 4);
            Handled::Ok
        }),
        ("ws2_32.dll", "inet_pton", |c| {
            c.ret_stdcall(0, 3);
            Handled::Ok
        }),    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

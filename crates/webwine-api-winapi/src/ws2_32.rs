//! ws2_32.dll — Winsock 2.
//!
//! Init, byte-order, and address parsing work. Socket I/O fails with
//! WSAECONNREFUSED / INVALID_SOCKET so apps reach their UI offline paths.

use super::{ApiContext, Handled, WinApiRegistry};

const SOCKET_ERROR: u32 = 0xFFFF_FFFF;
const INVALID_SOCKET: u32 = 0xFFFF_FFFF;
const WSAECONNREFUSED: u32 = 10061;
const WSAENOTSOCK: u32 = 10038;
const WSAEFAULT: u32 = 10014;
const WSAEINVAL: u32 = 10022;
const WSAHOST_NOT_FOUND: u32 = 11001;

pub fn register(r: &mut WinApiRegistry) {
    let fns: &[(&str, &str, super::HandlerFn)] = &[
        ("ws2_32.dll", "WSAStartup", wsa_startup),
        ("ws2_32.dll", "WSACleanup", wsa_cleanup),
        ("ws2_32.dll", "WSAGetLastError", wsa_get_last_error),
        ("ws2_32.dll", "WSASetLastError", wsa_set_last_error),
        ("ws2_32.dll", "WSACreateEvent", wsa_create_event),
        ("ws2_32.dll", "WSACloseEvent", wsa_close_event),
        ("ws2_32.dll", "WSAAsyncSelect", wsa_async_select),
        ("ws2_32.dll", "WSAEventSelect", wsa_event_select),
        ("ws2_32.dll", "WSAIoctl", wsa_ioctl),
        ("ws2_32.dll", "WSAAddressToStringA", wsa_address_to_string_a),
        ("ws2_32.dll", "WSAStringToAddressA", wsa_string_to_address_a),
        ("ws2_32.dll", "socket", socket_fn),
        ("ws2_32.dll", "WSASocketA", wsa_socket_a),
        ("ws2_32.dll", "WSASocketW", wsa_socket_a),
        ("ws2_32.dll", "closesocket", close_socket),
        ("ws2_32.dll", "connect", connect_fn),
        ("ws2_32.dll", "bind", bind_fn),
        ("ws2_32.dll", "listen", listen_fn),
        ("ws2_32.dll", "accept", accept_fn),
        ("ws2_32.dll", "send", send_fn),
        ("ws2_32.dll", "recv", recv_fn),
        ("ws2_32.dll", "sendto", sendto_fn),
        ("ws2_32.dll", "recvfrom", recvfrom_fn),
        ("ws2_32.dll", "shutdown", shutdown_fn),
        ("ws2_32.dll", "select", select_fn),
        ("ws2_32.dll", "ioctlsocket", ioctl_socket),
        ("ws2_32.dll", "getsockname", get_sock_name),
        ("ws2_32.dll", "getpeername", get_peer_name),
        ("ws2_32.dll", "getsockopt", get_sock_opt),
        ("ws2_32.dll", "setsockopt", set_sock_opt),
        ("ws2_32.dll", "gethostname", get_host_name),
        ("ws2_32.dll", "gethostbyname", get_host_by_name),
        ("ws2_32.dll", "getaddrinfo", get_addr_info),
        ("ws2_32.dll", "freeaddrinfo", free_addr_info),
        ("ws2_32.dll", "GetAddrInfoW", get_addr_info),
        ("ws2_32.dll", "FreeAddrInfoW", free_addr_info),
        ("ws2_32.dll", "getnameinfo", get_name_info),
        ("ws2_32.dll", "inet_addr", inet_addr),
        ("ws2_32.dll", "inet_ntoa", inet_ntoa),
        ("ws2_32.dll", "htons", htons),
        ("ws2_32.dll", "ntohs", ntohs),
        ("ws2_32.dll", "htonl", htonl),
        ("ws2_32.dll", "ntohl", ntohl),
        ("ws2_32.dll", "__WSAFDIsSet", wsa_fd_is_set),
        ("ws2_32.dll", "WSAWaitForMultipleEvents", wsa_wait_for_multiple_events),
        ("ws2_32.dll", "WSAEnumNetworkEvents", wsa_enum_network_events),
        ("ws2_32.dll", "WSAGetOverlappedResult", wsa_get_overlapped_result),
        ("ws2_32.dll", "getservbyname", get_serv_by_name),
        ("ws2_32.dll", "getservbyport", get_serv_by_port),
        ("ws2_32.dll", "inet_ntop", inet_ntop),
        ("ws2_32.dll", "inet_pton", inet_pton),
    ];
    for &(dll, name, f) in fns {
        r.add(dll, name, f);
    }
}

fn set_wsa_error(c: &mut ApiContext, err: u32) {
    c.dll_state.insert("ws2.lasterror".into(), err);
}

fn wsa_startup(c: &mut ApiContext) -> Handled {
    // WSAStartup(wVersionRequested, lpWSAData)
    let data = c.arg(1);
    if data != 0 {
        // WSADATA: wVersion, wHighVersion, then description/systemStatus strings.
        let _ = c.memory.write_u16(data, 0x0202);
        let _ = c.memory.write_u16(data + 2, 0x0202);
        // szDescription at offset 4 (A: 257 bytes), szSystemStatus at 261.
        let desc = b"WebWINE Winsock 2.2\0";
        let _ = c.memory.write_bytes(data + 4, desc);
        let sys = b"Running\0";
        let _ = c.memory.write_bytes(data + 261, sys);
        // iMaxSockets / iMaxUdpDg / lpVendorInfo — leave zeroed.
    }
    c.dll_state.insert("ws2.started".into(), 1);
    set_wsa_error(c, 0);
    c.ret_stdcall(0, 2);
    Handled::Ok
}

fn wsa_cleanup(c: &mut ApiContext) -> Handled {
    c.dll_state.remove("ws2.started");
    set_wsa_error(c, 0);
    c.ret_stdcall(0, 0);
    Handled::Ok
}

fn wsa_get_last_error(c: &mut ApiContext) -> Handled {
    let e = c.dll_state.get("ws2.lasterror").copied().unwrap_or(0);
    c.ret_stdcall(e, 0);
    Handled::Ok
}

fn wsa_set_last_error(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, c.arg(0));
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn wsa_create_event(c: &mut ApiContext) -> Handled {
    // Returns WSAEVENT handle or NULL.
    let h = c
        .dll_state
        .entry("ws2.event_next".into())
        .or_insert(0x5500_0001);
    let ev = *h;
    *h = h.wrapping_add(1);
    c.dll_state.insert(format!("ws2.event.{ev}"), 1);
    c.ret_stdcall(ev, 0);
    Handled::Ok
}

fn wsa_close_event(c: &mut ApiContext) -> Handled {
    let h = c.arg(0);
    c.dll_state.remove(&format!("ws2.event.{h}"));
    c.ret_stdcall(1, 1);
    Handled::Ok
}

fn wsa_async_select(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAENOTSOCK);
    c.ret_stdcall(SOCKET_ERROR, 4);
    Handled::Ok
}

fn wsa_event_select(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAENOTSOCK);
    c.ret_stdcall(SOCKET_ERROR, 3);
    Handled::Ok
}

fn wsa_ioctl(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAENOTSOCK);
    c.ret_stdcall(SOCKET_ERROR, 9);
    Handled::Ok
}

fn wsa_address_to_string_a(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAEINVAL);
    c.ret_stdcall(SOCKET_ERROR, 5);
    Handled::Ok
}

fn wsa_string_to_address_a(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAEINVAL);
    c.ret_stdcall(SOCKET_ERROR, 5);
    Handled::Ok
}

fn socket_fn(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAECONNREFUSED);
    c.ret_stdcall(INVALID_SOCKET, 3);
    Handled::Ok
}

fn wsa_socket_a(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAECONNREFUSED);
    c.ret_stdcall(INVALID_SOCKET, 6);
    Handled::Ok
}

fn close_socket(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, 0);
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn connect_fn(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAECONNREFUSED);
    c.ret_stdcall(SOCKET_ERROR, 3);
    Handled::Ok
}

fn bind_fn(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAECONNREFUSED);
    c.ret_stdcall(SOCKET_ERROR, 3);
    Handled::Ok
}

fn listen_fn(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAENOTSOCK);
    c.ret_stdcall(SOCKET_ERROR, 2);
    Handled::Ok
}

fn accept_fn(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAENOTSOCK);
    c.ret_stdcall(INVALID_SOCKET, 3);
    Handled::Ok
}

fn send_fn(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAENOTSOCK);
    c.ret_stdcall(SOCKET_ERROR, 4);
    Handled::Ok
}

fn recv_fn(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAENOTSOCK);
    c.ret_stdcall(SOCKET_ERROR, 4);
    Handled::Ok
}

fn sendto_fn(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAENOTSOCK);
    c.ret_stdcall(SOCKET_ERROR, 6);
    Handled::Ok
}

fn recvfrom_fn(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAENOTSOCK);
    c.ret_stdcall(SOCKET_ERROR, 6);
    Handled::Ok
}

fn shutdown_fn(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, 0);
    c.ret_stdcall(0, 2);
    Handled::Ok
}

fn select_fn(c: &mut ApiContext) -> Handled {
    // No ready sockets.
    set_wsa_error(c, 0);
    c.ret_stdcall(0, 5);
    Handled::Ok
}

fn ioctl_socket(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, 0);
    c.ret_stdcall(0, 3);
    Handled::Ok
}

fn get_sock_name(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAENOTSOCK);
    c.ret_stdcall(SOCKET_ERROR, 3);
    Handled::Ok
}

fn get_peer_name(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAENOTSOCK);
    c.ret_stdcall(SOCKET_ERROR, 3);
    Handled::Ok
}

fn get_sock_opt(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, 0);
    c.ret_stdcall(0, 5);
    Handled::Ok
}

fn set_sock_opt(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, 0);
    c.ret_stdcall(0, 5);
    Handled::Ok
}

fn get_host_name(c: &mut ApiContext) -> Handled {
    // gethostname(name, namelen)
    let buf = c.arg(0);
    let len = c.arg(1) as usize;
    let name = b"webwine\0";
    if buf == 0 || len == 0 {
        set_wsa_error(c, WSAEFAULT);
        c.ret_stdcall(SOCKET_ERROR, 2);
        return Handled::Ok;
    }
    let n = name.len().min(len);
    let _ = c.memory.write_bytes(buf, &name[..n]);
    if n < name.len() {
        // Truncated — still success on Windows if null fits; force null.
        let _ = c.memory.write_u8(buf + (n as u32).saturating_sub(1), 0);
    }
    set_wsa_error(c, 0);
    c.ret_stdcall(0, 2);
    Handled::Ok
}

fn get_host_by_name(c: &mut ApiContext) -> Handled {
    // struct hostent* — no DNS; return NULL.
    set_wsa_error(c, WSAHOST_NOT_FOUND);
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn get_addr_info(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAHOST_NOT_FOUND);
    c.ret_stdcall(WSAHOST_NOT_FOUND, 4);
    Handled::Ok
}

fn free_addr_info(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 1);
    Handled::Ok
}

fn get_name_info(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAEINVAL);
    c.ret_stdcall(SOCKET_ERROR, 7);
    Handled::Ok
}

/// inet_addr("a.b.c.d") → network-order IPv4, or INADDR_NONE (0xFFFFFFFF).
fn inet_addr(c: &mut ApiContext) -> Handled {
    let p = c.arg(0);
    if p == 0 {
        c.ret_stdcall(0xFFFF_FFFF, 1);
        return Handled::Ok;
    }
    let s = c.cstr(p);
    let result = parse_ipv4(&s).unwrap_or(0xFFFF_FFFF);
    c.ret_stdcall(result, 1);
    Handled::Ok
}

fn parse_ipv4(s: &str) -> Option<u32> {
    let parts: Vec<&str> = s.trim().split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let mut bytes = [0u8; 4];
    for (i, p) in parts.iter().enumerate() {
        let v: u32 = p.parse().ok()?;
        if v > 255 {
            return None;
        }
        bytes[i] = v as u8;
    }
    // Network byte order as a little-endian host u32 load of big-endian bytes.
    Some(u32::from_be_bytes(bytes))
}

fn inet_ntoa(c: &mut ApiContext) -> Handled {
    // char* inet_ntoa(struct in_addr) — in_addr is passed by value (1 dword on x86).
    let addr = c.arg(0);
    let bytes = addr.to_be_bytes();
    let s = format!("{}.{}.{}.{}\0", bytes[0], bytes[1], bytes[2], bytes[3]);
    // Static buffer in CRT data page.
    const SLOT: u32 = 0x7FFC_0200;
    let _ = c.memory.ensure_mapped(SLOT, SLOT + 16);
    let _ = c.memory.write_bytes(SLOT, s.as_bytes());
    c.ret_stdcall(SLOT, 1);
    Handled::Ok
}

fn htons(c: &mut ApiContext) -> Handled {
    let v = c.arg(0) as u16;
    c.ret_stdcall(v.swap_bytes() as u32, 1);
    Handled::Ok
}

fn ntohs(c: &mut ApiContext) -> Handled {
    let v = c.arg(0) as u16;
    c.ret_stdcall(v.swap_bytes() as u32, 1);
    Handled::Ok
}

fn htonl(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(c.arg(0).swap_bytes(), 1);
    Handled::Ok
}

fn ntohl(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(c.arg(0).swap_bytes(), 1);
    Handled::Ok
}

fn wsa_fd_is_set(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 2);
    Handled::Ok
}

fn wsa_wait_for_multiple_events(c: &mut ApiContext) -> Handled {
    // WSA_WAIT_FAILED
    set_wsa_error(c, WSAEINVAL);
    c.ret_stdcall(0xFFFF_FFFF, 5);
    Handled::Ok
}

fn wsa_enum_network_events(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, 0);
    c.ret_stdcall(0, 3);
    Handled::Ok
}

fn wsa_get_overlapped_result(c: &mut ApiContext) -> Handled {
    set_wsa_error(c, WSAEINVAL);
    c.ret_stdcall(0, 5);
    Handled::Ok
}

fn get_serv_by_name(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 2);
    Handled::Ok
}

fn get_serv_by_port(c: &mut ApiContext) -> Handled {
    c.ret_stdcall(0, 2);
    Handled::Ok
}

fn inet_ntop(c: &mut ApiContext) -> Handled {
    // PCSTR inet_ntop(af, src, dst, size)
    let af = c.arg(0);
    let src = c.arg(1);
    let dst = c.arg(2);
    let size = c.arg(3) as usize;
    if af != 2 || src == 0 || dst == 0 || size < 8 {
        // AF_INET = 2
        set_wsa_error(c, WSAEFAULT);
        c.ret_stdcall(0, 4);
        return Handled::Ok;
    }
    let b0 = c.memory.read_u8(src).unwrap_or(0);
    let b1 = c.memory.read_u8(src + 1).unwrap_or(0);
    let b2 = c.memory.read_u8(src + 2).unwrap_or(0);
    let b3 = c.memory.read_u8(src + 3).unwrap_or(0);
    let s = format!("{b0}.{b1}.{b2}.{b3}");
    if s.len() + 1 > size {
        set_wsa_error(c, WSAEFAULT);
        c.ret_stdcall(0, 4);
        return Handled::Ok;
    }
    let _ = c.memory.write_bytes(dst, s.as_bytes());
    let _ = c.memory.write_u8(dst + s.len() as u32, 0);
    c.ret_stdcall(dst, 4);
    Handled::Ok
}

fn inet_pton(c: &mut ApiContext) -> Handled {
    // int inet_pton(af, src, dst)
    let af = c.arg(0);
    let src = c.arg(1);
    let dst = c.arg(2);
    if af != 2 || src == 0 || dst == 0 {
        c.ret_stdcall(0xFFFF_FFFF, 3); // -1
        return Handled::Ok;
    }
    let s = c.cstr(src);
    match parse_ipv4(&s) {
        Some(addr) => {
            // Write network-order bytes.
            let bytes = addr.to_be_bytes();
            let _ = c.memory.write_bytes(dst, &bytes);
            c.ret_stdcall(1, 3);
        }
        None => c.ret_stdcall(0, 3), // not a valid address
    }
    Handled::Ok
}

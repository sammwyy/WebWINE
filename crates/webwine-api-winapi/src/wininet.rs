//! wininet.dll — WinINet HTTP/session API.
//!
//! Browser sandbox: no real network from the guest. Sessions and URLs open as
//! fake handles so apps that only probe connectivity or open-and-close keep
//! running; reads return 0 bytes (EOF).

use super::{ApiContext, Handled, WinApiRegistry};
use webwine_api::winapi::context::ApiRuntimeEnv;

const ERROR_INVALID_HANDLE: u32 = 6;
const ERROR_INVALID_PARAMETER: u32 = 87;
const ERROR_INTERNET_NAME_NOT_RESOLVED: u32 = 12007;
const ERROR_INTERNET_CANNOT_CONNECT: u32 = 12029;

const TAG_SESSION: u32 = 0x7701_0000;
const TAG_CONNECT: u32 = 0x7702_0000;
const TAG_REQUEST: u32 = 0x7703_0000;
const TAG_URL: u32 = 0x7704_0000;

pub fn register(r: &mut WinApiRegistry) {
    r.add("wininet.dll", "InternetOpenA", |c| internet_open(c, false));
    r.add("wininet.dll", "InternetOpenW", |c| internet_open(c, true));
    r.add("wininet.dll", "InternetCloseHandle", internet_close_handle);
    r.add("wininet.dll", "InternetOpenUrlA", |c| internet_open_url(c, false));
    r.add("wininet.dll", "InternetOpenUrlW", |c| internet_open_url(c, true));
    r.add("wininet.dll", "InternetConnectA", |c| internet_connect(c, false));
    r.add("wininet.dll", "InternetConnectW", |c| internet_connect(c, true));
    r.add("wininet.dll", "InternetReadFile", internet_read_file);
    r.add("wininet.dll", "InternetWriteFile", internet_write_file);
    r.add("wininet.dll", "InternetQueryDataAvailable", internet_query_data_available);
    r.add("wininet.dll", "InternetSetOptionA", internet_set_option);
    r.add("wininet.dll", "InternetSetOptionW", internet_set_option);
    r.add("wininet.dll", "InternetQueryOptionA", internet_query_option);
    r.add("wininet.dll", "InternetQueryOptionW", internet_query_option);
    r.add("wininet.dll", "HttpOpenRequestA", |c| http_open_request(c, false));
    r.add("wininet.dll", "HttpOpenRequestW", |c| http_open_request(c, true));
    r.add("wininet.dll", "HttpSendRequestA", |c| http_send_request(c, false));
    r.add("wininet.dll", "HttpSendRequestW", |c| http_send_request(c, true));
    r.add("wininet.dll", "HttpQueryInfoA", |c| http_query_info(c, false));
    r.add("wininet.dll", "HttpQueryInfoW", |c| http_query_info(c, true));
    r.add(
        "wininet.dll",
        "InternetGetConnectedState",
        internet_get_connected_state,
    );
    r.add(
        "wininet.dll",
        "InternetGetConnectedStateExA",
        internet_get_connected_state_ex,
    );
    r.add(
        "wininet.dll",
        "InternetGetConnectedStateExW",
        internet_get_connected_state_ex,
    );
    r.add("wininet.dll", "InternetCheckConnectionA", internet_check_connection);
    r.add("wininet.dll", "InternetCheckConnectionW", internet_check_connection);
    r.add("wininet.dll", "InternetCrackUrlA", |c| internet_crack_url(c, false));
    r.add("wininet.dll", "InternetCrackUrlW", |c| internet_crack_url(c, true));
    r.add("wininet.dll", "InternetSetStatusCallbackA", internet_set_status_callback);
    r.add("wininet.dll", "InternetSetStatusCallbackW", internet_set_status_callback);
    r.add("wininet.dll", "InternetSetStatusCallback", internet_set_status_callback);
}

fn next_handle(c: &mut ApiContext, tag: u32) -> u32 {
    let n = c.dll_state.entry("wininet.next".into()).or_insert(1);
    let id = *n;
    *n = n.wrapping_add(1).max(1);
    tag | (id & 0x0000_FFFF)
}

fn is_live(c: &ApiContext, h: u32) -> bool {
    h != 0 && c.dll_state.contains_key(&format!("wininet.h.{h}"))
}

fn internet_open(c: &mut ApiContext, wide: bool) -> Handled {
    // InternetOpen(agent, access, proxy, proxy_bypass, flags) → HINTERNET
    let agent = c.arg(0);
    let _ = wide;
    let h = next_handle(c, TAG_SESSION);
    c.dll_state.insert(format!("wininet.h.{h}"), 1);
    if agent != 0 {
        let name = if wide {
            c.wstr(agent)
        } else {
            c.cstr(agent)
        };
        c.logs.log(
            webwine_api::logs::LogLevel::Trace,
            "api",
            &format!("InternetOpen agent={name:?}"),
            Some(c.pid),
        );
    }
    c.return_stdcall(h, 5);
    Handled::Ok
}

fn internet_close_handle(c: &mut ApiContext) -> Handled {
    let h = c.arg(0);
    if h == 0 {
        c.cpu.last_error = ERROR_INVALID_HANDLE;
        c.return_stdcall(0, 1);
        return Handled::Ok;
    }
    c.dll_state.remove(&format!("wininet.h.{h}"));
    c.return_stdcall(1, 1);
    Handled::Ok
}

fn internet_open_url(c: &mut ApiContext, wide: bool) -> Handled {
    // InternetOpenUrl(hInternet, url, headers, headers_len, flags, context) → HINTERNET
    let session = c.arg(0);
    let url_ptr = c.arg(1);
    if !is_live(c, session) {
        c.cpu.last_error = ERROR_INVALID_HANDLE;
        c.return_stdcall(0, 6);
        return Handled::Ok;
    }
    let url = if url_ptr == 0 {
        String::new()
    } else if wide {
        c.wstr(url_ptr)
    } else {
        c.cstr(url_ptr)
    };
    if url.is_empty() {
        c.cpu.last_error = ERROR_INVALID_PARAMETER;
        c.return_stdcall(0, 6);
        return Handled::Ok;
    }
    // Offline: still hand back a handle so callers can Close it; ReadFile → EOF.
    let h = next_handle(c, TAG_URL);
    c.dll_state.insert(format!("wininet.h.{h}"), 1);
    c.dll_state.insert(format!("wininet.url.{h}"), url.len() as u32);
    c.return_stdcall(h, 6);
    Handled::Ok
}

fn internet_connect(c: &mut ApiContext, wide: bool) -> Handled {
    // InternetConnect(hInternet, server, port, user, pass, service, flags, ctx) → 8 args
    let session = c.arg(0);
    let server = c.arg(1);
    if !is_live(c, session) {
        c.cpu.last_error = ERROR_INVALID_HANDLE;
        c.return_stdcall(0, 8);
        return Handled::Ok;
    }
    if server == 0 {
        c.cpu.last_error = ERROR_INVALID_PARAMETER;
        c.return_stdcall(0, 8);
        return Handled::Ok;
    }
    let _host = if wide {
        c.wstr(server)
    } else {
        c.cstr(server)
    };
    // No network: fail with CANNOT_CONNECT (apps often fall back to offline UI).
    c.cpu.last_error = ERROR_INTERNET_CANNOT_CONNECT;
    c.return_stdcall(0, 8);
    Handled::Ok
}

fn internet_read_file(c: &mut ApiContext) -> Handled {
    // BOOL InternetReadFile(hFile, buffer, numberOfBytesToRead, numberOfBytesRead)
    let h = c.arg(0);
    let nread = c.arg(3);
    if !is_live(c, h) {
        c.cpu.last_error = ERROR_INVALID_HANDLE;
        c.return_stdcall(0, 4);
        return Handled::Ok;
    }
    // EOF: success with 0 bytes.
    if nread != 0 {
        c.write_u32(nread, 0);
    }
    c.return_stdcall(1, 4);
    Handled::Ok
}

fn internet_write_file(c: &mut ApiContext) -> Handled {
    let h = c.arg(0);
    let nwritten = c.arg(3);
    if !is_live(c, h) {
        c.cpu.last_error = ERROR_INVALID_HANDLE;
        c.return_stdcall(0, 4);
        return Handled::Ok;
    }
    if nwritten != 0 {
        c.write_u32(nwritten, 0);
    }
    c.cpu.last_error = ERROR_INTERNET_CANNOT_CONNECT;
    c.return_stdcall(0, 4);
    Handled::Ok
}

fn internet_query_data_available(c: &mut ApiContext) -> Handled {
    let h = c.arg(0);
    let out = c.arg(1);
    if !is_live(c, h) {
        c.cpu.last_error = ERROR_INVALID_HANDLE;
        c.return_stdcall(0, 2);
        return Handled::Ok;
    }
    if out != 0 {
        c.write_u32(out, 0);
    }
    c.return_stdcall(1, 2);
    Handled::Ok
}

fn internet_set_option(c: &mut ApiContext) -> Handled {
    // BOOL InternetSetOption(hInternet, option, buffer, bufferLength)
    c.return_stdcall(1, 4);
    Handled::Ok
}

fn internet_query_option(c: &mut ApiContext) -> Handled {
    let buflen = c.arg(3);
    if buflen != 0 {
        // Report required size 0 when we have nothing.
        let needed = c.read_u32(buflen);
        let _ = needed;
        c.write_u32(buflen, 0);
    }
    c.return_stdcall(1, 4);
    Handled::Ok
}

fn http_open_request(c: &mut ApiContext, wide: bool) -> Handled {
    // HttpOpenRequest(hConnect, verb, object, version, referer, accept, flags, ctx) → 8
    let connect = c.arg(0);
    let _ = wide;
    if !is_live(c, connect) {
        // Also accept when connect failed earlier — some apps still call this.
        c.cpu.last_error = ERROR_INVALID_HANDLE;
        c.return_stdcall(0, 8);
        return Handled::Ok;
    }
    let h = next_handle(c, TAG_REQUEST);
    c.dll_state.insert(format!("wininet.h.{h}"), 1);
    c.return_stdcall(h, 8);
    Handled::Ok
}

fn http_send_request(c: &mut ApiContext, _wide: bool) -> Handled {
    // BOOL HttpSendRequest(hRequest, headers, headersLen, optional, optionalLen)
    let h = c.arg(0);
    if !is_live(c, h) {
        c.cpu.last_error = ERROR_INVALID_HANDLE;
        c.return_stdcall(0, 5);
        return Handled::Ok;
    }
    c.cpu.last_error = ERROR_INTERNET_NAME_NOT_RESOLVED;
    c.return_stdcall(0, 5);
    Handled::Ok
}

fn http_query_info(c: &mut ApiContext, wide: bool) -> Handled {
    // BOOL HttpQueryInfo(hRequest, infoLevel, buffer, bufferLength, index)
    let h = c.arg(0);
    let buflen = c.arg(3);
    let _ = wide;
    if !is_live(c, h) {
        c.cpu.last_error = ERROR_INVALID_HANDLE;
        c.return_stdcall(0, 5);
        return Handled::Ok;
    }
    if buflen != 0 {
        c.write_u32(buflen, 0);
    }
    c.return_stdcall(0, 5);
    Handled::Ok
}

fn internet_get_connected_state(c: &mut ApiContext) -> Handled {
    // BOOL InternetGetConnectedState(lpdwFlags, dwReserved)
    // Report "not connected" so apps take offline paths.
    let flags = c.arg(0);
    if flags != 0 {
        c.write_u32(flags, 0);
    }
    c.return_stdcall(0, 2);
    Handled::Ok
}

fn internet_get_connected_state_ex(c: &mut ApiContext) -> Handled {
    // BOOL InternetGetConnectedStateEx(lpdwFlags, lpszConnectionName, dwNameLen, dwReserved)
    let flags = c.arg(0);
    if flags != 0 {
        c.write_u32(flags, 0);
    }
    c.return_stdcall(0, 4);
    Handled::Ok
}

fn internet_check_connection(c: &mut ApiContext) -> Handled {
    // BOOL InternetCheckConnection(url, flags, reserved)
    c.return_stdcall(0, 3);
    Handled::Ok
}

fn internet_crack_url(c: &mut ApiContext, wide: bool) -> Handled {
    // BOOL InternetCrackUrl(url, urlLength, flags, lpUrlComponents)
    // Minimal: succeed without filling components if structure is present.
    let url = c.arg(0);
    let components = c.arg(3);
    if url == 0 || components == 0 {
        c.cpu.last_error = ERROR_INVALID_PARAMETER;
        c.return_stdcall(0, 4);
        return Handled::Ok;
    }
    let _s = if wide {
        c.wstr(url)
    } else {
        c.cstr(url)
    };
    // Leave URL_COMPONENTS as-is (caller sets dwStructSize / buffer sizes).
    c.return_stdcall(1, 4);
    Handled::Ok
}

fn internet_set_status_callback(c: &mut ApiContext) -> Handled {
    // INTERNET_STATUS_CALLBACK InternetSetStatusCallback(hInternet, callback)
    // Return previous callback (NULL).
    c.return_stdcall(0, 2);
    Handled::Ok
}

#[allow(dead_code)]
fn _tag_connect() -> u32 {
    TAG_CONNECT
}

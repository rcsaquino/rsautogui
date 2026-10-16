//! Wayland screenshots via `xdg-desktop-portal` (with a GNOME Shell fallback).
//!
//! The freedesktop `Screenshot` portal takes a fullscreen still image without
//! any user interaction and hands back a `file://` URI, which we decode with
//! the built-in PNG decoder. This works on GNOME, KDE and wlroots compositors
//! as long as `xdg-desktop-portal` (plus a backend like
//! `xdg-desktop-portal-gnome/-kde/-wlr/-gtk`) is running — the standard setup
//! on most Wayland desktops.

use std::io;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use crate::image::png::decode_png;
use crate::image::Image;

use super::dbus::{self, DVal, Writer};

static TOKEN_COUNTER: AtomicU32 = AtomicU32::new(0);

fn is_missing_service(e: &io::Error) -> bool {
    let m = e.to_string();
    m.contains("ServiceUnknown")
        || m.contains("UnknownMethod")
        || m.contains("UnknownInterface")
        || m.contains("UnknownObject")
}

/// Captures the whole primary output. `fallback_size` is only used by the
/// GNOME Shell fallback (which needs an explicit rectangle).
pub(crate) fn capture_fullscreen(fallback_size: (u16, u16)) -> io::Result<Image> {
    let mut bus = dbus::connect()?;
    match portal_screenshot(&mut bus) {
        Ok(img) => Ok(img),
        Err(e) if is_missing_service(&e) => gnome_screenshot(&mut bus, fallback_size),
        Err(e) => Err(e),
    }
}

fn portal_screenshot(bus: &mut dbus::Bus) -> io::Result<Image> {
    let token = format!(
        "rsautogui{}_{}",
        std::process::id(),
        TOKEN_COUNTER.fetch_add(1, Ordering::SeqCst)
    );
    let sender_suffix = bus.sender.trim_start_matches(':').replace('.', "_");
    let req_path = format!("/org/freedesktop/portal/desktop/request/{sender_suffix}/{token}");

    bus.add_match(&format!(
        "type='signal',sender='org.freedesktop.portal.Desktop',\
         interface='org.freedesktop.portal.Request',member='Response',path='{req_path}'"
    ))?;

    let mut args = Writer::new();
    args.string(""); // parent_window: none
    args.array(8, |w| {
        w.dict_entry(|w| {
            w.string("handle_token");
            w.variant("s", |w| w.string(&token));
        });
    });
    let arg_body = args.into_bytes();

    let (reply, _) = bus.call(
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Screenshot",
        "Screenshot",
        "sa{sv}",
        &arg_body,
    )?;
    let handle = match reply.first() {
        Some(DVal::ObjPath(p)) => p.clone(),
        other => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected portal Screenshot reply: {other:?}"),
            ));
        }
    };
    if handle != req_path {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("portal used an unexpected request handle: {handle}"),
        ));
    }

    let deadline = Instant::now() + Duration::from_secs(30);
    let msg = bus.wait_signal(deadline, |m| m.path == req_path && m.member == "Response")?;
    if msg.body.len() != 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "malformed portal Response signal",
        ));
    }
    let code = msg.body[0].as_u32().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "malformed portal Response code")
    })?;
    if code != 0 {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("screenshot portal request failed or was denied (response {code})"),
        ));
    }
    let uri = msg.body[1]
        .dict_get("uri")
        .and_then(|v| v.variant_inner())
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "portal Response did not contain an image uri",
            )
        })?;
    let path = uri_to_path(uri)?;
    let bytes = std::fs::read(&path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("cannot read portal screenshot file {path}: {e}"),
        )
    })?;
    decode_png(&bytes)
}

fn gnome_screenshot(bus: &mut dbus::Bus, size: (u16, u16)) -> io::Result<Image> {
    let tmp = format!(
        "/tmp/rsautogui-{}-{}.png",
        std::process::id(),
        TOKEN_COUNTER.fetch_add(1, Ordering::SeqCst)
    );
    let mut args = Writer::new();
    args.i32_(0);
    args.i32_(0);
    args.i32_(size.0 as i32);
    args.i32_(size.1 as i32);
    args.bool_(false); // flash
    args.string(&tmp);
    let body = args.into_bytes();

    let (reply, _) = bus.call(
        "org.gnome.Shell",
        "/org/gnome/Shell/Screenshot",
        "org.gnome.Shell.Screenshot",
        "ScreenshotArea",
        "iiiibs",
        &body,
    )?;
    if reply.len() != 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "malformed GNOME screenshot reply",
        ));
    }
    let ok = reply[0].as_bool().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "malformed GNOME screenshot reply",
        )
    })?;
    let used = reply[1].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "malformed GNOME screenshot reply",
        )
    })?;
    if !ok {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "GNOME Shell screenshot call reported failure",
        ));
    }
    let path = if used.is_empty() { tmp.clone() } else { used.to_owned() };
    let bytes = std::fs::read(&path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("cannot read GNOME screenshot file {path}: {e}"),
        )
    })?;
    let _ = std::fs::remove_file(&path);
    decode_png(&bytes)
}

fn uri_to_path(uri: &str) -> io::Result<String> {
    let p = uri.strip_prefix("file://").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("portal returned a non-file uri: {uri}"),
        )
    })?;
    let p = p.strip_prefix("localhost").unwrap_or(p);
    percent_decode(p)
}

fn percent_decode(s: &str) -> io::Result<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            if i + 2 >= b.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "bad percent-encoding in file uri",
                ));
            }
            let hex = |c: u8| match c {
                b'0'..=b'9' => Some(c - b'0'),
                b'a'..=b'f' => Some(c - b'a' + 10),
                b'A'..=b'F' => Some(c - b'A' + 10),
                _ => None,
            };
            match (hex(b[i + 1]), hex(b[i + 2])) {
                (Some(h), Some(l)) => out.push(h * 16 + l),
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "bad percent-encoding in file uri",
                    ))
                }
            }
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("bad file uri: {e}")))
}

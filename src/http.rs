//! Just enough HTTP/1.1 to answer GETs behind Cloud Run, which
//! terminates TLS and speaks plain HTTP/1.1 to the container.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

pub struct Response {
    pub status: u16,
    pub headers: Vec<(&'static str, &'static str)>,
    pub body: Vec<u8>,
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        404 => "Not Found",
        _ => "Bad Request",
    }
}

/// Request target of the first line, or None if this is not a GET.
fn target(reader: &mut impl BufRead) -> Option<String> {
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let mut parts = line.split_whitespace();
    (parts.next()? == "GET").then(|| parts.next())?.map(str::to_string)
}

fn write(stream: &mut TcpStream, response: &Response) -> std::io::Result<()> {
    write!(stream, "HTTP/1.1 {} {}\r\n", response.status, reason(response.status))?;
    for (name, value) in &response.headers {
        write!(stream, "{name}: {value}\r\n")?;
    }
    write!(stream, "content-length: {}\r\n", response.body.len())?;
    write!(stream, "connection: close\r\n\r\n")?;
    stream.write_all(&response.body)
}

fn handle(mut stream: TcpStream, route: &dyn Fn(&str) -> Response) {
    let mut reader = BufReader::new(&stream);
    let response = match target(&mut reader) {
        Some(path) => route(&path),
        None => Response { status: 400, headers: vec![], body: vec![] },
    };
    // Drain the headers so the client doesn't see a reset before the reply.
    for line in reader.by_ref().lines() {
        if line.map_or(true, |l| l.is_empty()) {
            break;
        }
    }
    let _ = write(&mut stream, &response);
}

pub fn serve(port: u16, route: impl Fn(&str) -> Response + Send + Sync + 'static) -> ! {
    let listener = TcpListener::bind(("0.0.0.0", port)).expect("bind");
    let route = std::sync::Arc::new(route);
    loop {
        if let Ok((stream, _)) = listener.accept() {
            let route = route.clone();
            thread::spawn(move || handle(stream, &*route));
        }
    }
}

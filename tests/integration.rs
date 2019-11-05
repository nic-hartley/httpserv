use std::{
  thread::spawn,
  collections::HashMap,
  net::{TcpStream, ToSocketAddrs},
  sync::Once,
};

extern crate httpserv;
use httpserv::*;

static SETUP: Once = Once::new();

fn setup_httpserv(host_at: &'static str) {
  SETUP.call_once(move || {
    spawn(move || {
      Httpserv::new(Config {
        root: "./tests/webroot".into(),
        hostname: host_at.into(),
        mappings: HashMap::new(),
        log: false,
      }).expect("Failed to start httpserv").run();
    });
  });
}

fn request<A: ToSocketAddrs>(to: A, url: &str) -> String {
  use std::io::{Read, Write};

  let mut stream = TcpStream::connect(to).expect("failed to connect");
  // When we need to send headers, maybe just trim off that last \n?
  // then the caller can send it on its own when ready
  write!(stream, "GET {} HTTP/1.1\n\n", url).expect("failed to write");
  let mut resp = String::new();
  stream.read_to_string(&mut resp).expect("failed to get response");
  resp
}

fn strip_headers(mut resp: String, ctype: &str, len: usize) -> (String, String) {
  let required_headers = vec![
    "Cache-Control: no-cache".into(),
    "Connection: close".into(),
    format!("Content-Type: {}; charset=UTF-8", ctype),
    format!("Content-Length: {}", len),
  ];

  let first_nl = resp.find("\n").expect("bad response format");
  let mut headers = resp.split_off(first_nl).split_off(1);
  let first_line = resp; // after split_off

  let last_nl = headers.find("\n\n").expect("bad response format");
  let body = headers.split_off(last_nl).split_off(2);

  let got_headers = headers.split_terminator('\n').collect::<Vec<_>>();
  println!("{:?}", first_line);
  println!("{:?}", got_headers);
  for req_header in required_headers.into_iter() {
    assert!(got_headers.contains(&&req_header[..]), "missing required header: {}", req_header);
  }

  (first_line, body)
}

#[test]
fn test_404() {
  setup_httpserv("localhost:8080");
  let response = request("localhost:8080", "/nonexistent_asdkjakdjd");
  let (first, _) = strip_headers(response, "text/plain", 0);
  assert_eq!(first, "HTTP/1.1 404 Not Found", "wrong status reply");
}

#[test]
fn test_index() {
  println!("{:?}", std::env::current_dir());
  setup_httpserv("localhost:8080");
  let response = request("localhost:8080", "/");
  let (first, body) = strip_headers(response, "text/plain", 2);
  assert_eq!(first, "HTTP/1.1 200 OK", "wrong status reply");
  assert_eq!(body, "1\n", "wrong body");
}

#[test]
fn test_file() {
  println!("{:?}", std::env::current_dir());
  setup_httpserv("localhost:8080");
  let response = request("localhost:8080", "/file");
  let (first, body) = strip_headers(response, "text/plain", 2);
  assert_eq!(first, "HTTP/1.1 200 OK", "wrong status reply");
  assert_eq!(body, "2\n", "wrong body");
}

#[test]
fn test_subdir_redirect() {
  println!("{:?}", std::env::current_dir());
  setup_httpserv("localhost:8080");
  let response = request("localhost:8080", "/subdir");
  let (first, _) = strip_headers(response, "text/plain", 0);
  assert_eq!(first, "HTTP/1.1 301 Moved Permanently", "wrong status reply");
}

#[test]
fn test_subdir() {
  println!("{:?}", std::env::current_dir());
  setup_httpserv("localhost:8080");
  let response = request("localhost:8080", "/subdir/");
  let (first, body) = strip_headers(response, "text/plain", 2);
  assert_eq!(first, "HTTP/1.1 200 OK", "wrong status reply");
  assert_eq!(body, "3\n", "wrong body");
}

#[test]
fn test_subdir_file() {
  println!("{:?}", std::env::current_dir());
  setup_httpserv("localhost:8080");
  let response = request("localhost:8080", "/subdir/file");
  let (first, body) = strip_headers(response, "text/plain", 2);
  assert_eq!(first, "HTTP/1.1 200 OK", "wrong status reply");
  assert_eq!(body, "4\n", "wrong body");
}

/*
tests left to do:
- URL with ..
- URLs not starting with /
- symlinks
*/

use std::{io, net, time::Instant};

mod cfg;
pub use cfg::Config;
mod http;
use http::*;

#[derive(Debug)]
pub enum HttpservFail {
  Io(io::Error),
  Request(ReqFail),
}
impl From<io::Error> for HttpservFail {
  fn from(ioe: io::Error) -> Self {
    Self::Io(ioe)
  }
}
impl From<ReqFail> for HttpservFail {
  fn from(rf: ReqFail) -> Self {
    Self::Request(rf)
  }
}
type Result<T> = std::result::Result<T, HttpservFail>;

pub struct Httpserv {
  cfg: Config,
  listener: net::TcpListener,
}

impl Httpserv {
  pub fn new(cfg: Config) -> Result<Httpserv> {
    let listener = net::TcpListener::bind(&cfg.hostname)?;
    Ok(Httpserv { cfg, listener })
  }

  pub fn config(&self) -> &Config {
    &self.cfg
  }

  fn respond_one(&self, conn: io::Result<net::TcpStream>) -> Result<()> {
    // just ignore failed connections
    let begin = Instant::now();
    let mut conn = match conn {
      Ok(s) => s,
      Err(e) => return Err(e.into()),
    };
    let request = http::Request::load(&mut conn)?;
    if self.cfg.log {
      print!("Serving /{}", request.path)
    }
    let response = http::Response::to(request, &self.cfg)?;
    if self.cfg.log {
      print!(" with {}", response.code());
    }
    response.write(conn)?;
    if self.cfg.log {
      println!(" in {}ms", (Instant::now() - begin).as_micros());
    }
    Ok(())
  }

  pub fn run(&mut self) {
    for conn in self.listener.incoming() {
      let _ = self.respond_one(conn);
    }
  }

  pub fn run_to_fail(&mut self) -> Result<()> {
    for conn in self.listener.incoming() {
      self.respond_one(conn)?;
    }
    Ok(())
  }
}

# httpserv

A tiny, zero-dependency HTTP fileserver, meant for local development of HTML.
It's not the absolute fastest, it's not the absolute smallest executable, it's
not the most featureful, but it's incredibly simple to deploy and does almost
nothing but just serve the file you name over HTTP. Specifically, all it does
is:

- Parse the URL to find the local filepath
- Make sure that URL doesn't contain `..`s
- If that filepath points to a directory, add `/index.html`
- Use the extension to figure out the `Content-Type`
- Send the file back

Because of its simplicity, it's incredibly quick to install, quick to start,
and quick to respond

Planned tasks can be seen [in the issues][gh-issues]

Planned non-features include:

- Any dependencies by default (behind a feature gate is fine)
- Maximizing response speed where it would cost ease of use or setup speed
- Anything that gives httpserv a noticeable delay
- Anything targeted at better production suitability

 [gh-issues]: https://github.com/nic-hartley/httpserv/issues

## Install

```sh
cargo install httpserv
```

That's it. Assuming the Cargo bin directory is on your path, you can now call
`httpserv` from your command line. For directions on installing Cargo, please
see [rustup.rs].

On WSL, you may need to call `cargo.exe` and `httpserv.exe` instead, depending
on if you've got Rust installed on the Windows or WSL side of things.

 [rustup.rs]: https://rustup.rs/

## Usage

All arguments are optional -- if you want to serve your current directory on
`localhost:8080` with the default mappings, you can just type `httpserv` and
hit enter. Otherwise:

```sh
httpserv [directory] [listen] [mappings...]
```

- `directory`: Where to look for files to serve. Defaults to `.`
- `listen`: The host/port to listen on, as expected by the `FromStr` impl for
  `SocketAddr`. Defaults to `localhost:8080`
- `mappings...`: Any additional mappings from [file extension][ext] to MIME
  types, besides the defaults. Anything specified here which matches the same
  extension as a default will override the default MIME type. The format is
  `extension=MIME`, with no leading `.` on the extension.

 [ext]: https://doc.rust-lang.org/std/path/struct.Path.html#method.extension

## Known issues

Because this is meant for local development and not production use, there are
some issues which I haven't bothered to fix. In general, the reason why boils
down to httpserv being meant to aid local development. If you're using it in
any situation where you can't restart it at will, you're doing it very, *very*
wrong.

- Requests with absurdly long URLs or absurd numbers of headers can cause the
  process to hang or crash
- Requests are processed serially, so if enough are received at once, some may
  not get responses for a while; however, the time to parse any single request
  is so fast that it's not an issue normally
- If a file is changed between when the HTTP headers are sent and when the
  rest of the body is sent, the reported `Content-Length` will be incorrect,
  so the browser may truncate the content or display an error.
- A malicious actor could send a partial request (e.g. never ending the header)
  and lock up the server.

# httpserv

A tiny, zero-dependency HTTP fileserver, meant for local development of HTML.
It's not the absolute fastest, it's not the absolute smallest executable, it's
not the most featureful, but it's incredibly simple to deploy and does almost
nothing but just serve the file you name over HTTP. Specifically, all it does
is:

- Parse the URL to find the local filepath
- If that filepath points to a directory, add `/index.html`
- Use the extension to figure out the `Content-Type`
- Send the file back

Because of its simplicity, it's incredibly quick to install, quick to start,
and quick to respond

Planned tasks in no particular order include:

- General code cleanup
- Ensure the `..` block works as expected with other browsers
- Better error handling, so errors are less likely to crash the entire program
- Supporting percent-encoding in URLs
- More easily customizable mappings, to support extensions with `.`s in them
- Multithreading, maybe feature-gated, useful for serving lots of little files
- Respecting `Accept` headers
- Support for `Content-Encoding`, especially `gzip` (for pre-`gzip`'d files)
- Feature-gated TLS support, useful for testing sites that make requests to
  HTTPS sites.
- Feature-gated advanced option parsing with getopts or clap
- Feature-gated Markdown parsing

Planned non-features include:

- Any dependencies by default (behind a feature gate is fine)
- Maximizing response speed where it would cost ease of use or setup speed
- Anything that gives httpserv a noticeable delay

# Install

> Note: As this crate hasn't been published yet, I haven't tested this yet. I
> will soon.

```
cargo install httpserv
```

That's it. Assuming the Cargo bin directory is on your path, you can now call
`httpserv` from your command line.

# Usage

All arguments are optional -- if you want to serve your current directory on
`localhost:8080` with the default mappings, you can just type `httpserv` and
hit enter. Otherwise:

```
httpserv [directory] [listen] [mappings...]
```

* `directory`: Where to look for files to serve. Defaults to `.`
* `listen`: The host/port to listen on, as expected by the `FromStr` impl for
  `SocketAddr`. Defaults to `localhost:8080`
* `mappings...`: Any additional mappings from [file extension][ext] to MIME
  types, besides the defaults. Anything specified here which matches the same
  extension as a default will override the default MIME type.

 [ext]: https://doc.rust-lang.org/std/path/struct.Path.html#method.extension

# Known issues

Because this is meant for local development and not production uses, there are
some issues which I haven't bothered to fix. In general, the reason why boils
down to httpserv being meant to aid local development. If you're using it for
anything critical, you're doing it _very_ wrong and should get your hands on a
webserver actually made to be used in production.

- Requests with absurdly long URLs or absurd numbers of headers can cause the
  process to hang or crash
- Requests are processed serially, so if enough are received at once, some may
  not get responses for a while; however, the time to parse any single request
  is so fast that it's not an issue normally
- If a file is changed between when the HTTP headers are sent and when the
  rest of the body is sent, the reported `Content-Length` will be incorrect,
  so the browser may truncate the content or display an error.

# Dolores

**WIP: Works, but there is still a lot rough edges**

Local development HTTPS proxy server meant to simplify working with multi-domain
applications by serving each application on separate domain under `.localhost`
TLD.

## Usage

First we need to run our master server that will proxy all requests to the
separate applications:

```sh
sudo dolores serve
```

Now, as **unprivileged user** we can run:

```sh
dolores run --name my-app <your command for starting server>
```

For example

```sh
dolores run --name foo mix phx.server
```

And your application will receive socket to listen on (passed as FD3). It is
provided to your application in the same way that systemd socket activation
works. In short there are 3 environment variables:

- `LISTEN_FDS` - amount of file descriptors passed in. These are FD 3 up to
  FD 3 + `LISTEN_FDS`
- `LISTEN_FDNAMES` - `:` (colon) separated list of FD names used to differentiate
  between them
- `LISTEN_PID` - PID of the process that the FD are meant for

With current PoC implementation you can assume that there will be only one
FD passes and it will be FD 3.

Now you should be able to visit your application on <https://foo.localhost>.

## Goals

- [x] Listen on HTTPS requests and dispatch requests to given application
- [x] Passthrough proxy
- [x] TLS terminating proxy
- [ ] On-the-fly generation of TLS certificates (partially supported, only
  self-signed certs are supported for now)
- [ ] Registration of external ports
- [ ] Built-in ACME server for passthrough services
- [ ] Create page presenting all registered applications
- [ ] Provide Prometheus metrics for the proxy server
- [ ] Collect Prometheus metrics for all running applications

## Non-goals

- Performance - this tool is meant to be a development utility, performance
  improvements that could hurt usability are no go.
- Production-grade load balancing - for the same reason as above. Securing
  everything, performance tuning, etc.

## License

MIT

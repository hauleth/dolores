# Dolores

**WIP: Nothing works right now**

Local development HTTPS proxy server meant to simplify working with multi-domain
applications by serving each application on separate domain under `.localhost.`
TLD.

## Usage

First we need to run our master server that will proxy all requests to the
separate applications:

```sh
sudo systemctl start dolores
```

Now, as **unprivileged user** we can run:

```sh
dolores run <your command for starting server>
```

For example

```sh
dolores run mix phx.server
```

And your application will receive socket to listen on (passed as FD3).

Now you should be able to visit your application on <https://mix.localhost>.

## Goals

- [ ] Listen on HTTPS requests and dispatch requests to given application
- [ ] Provide Prometheus metrics for the proxy server
- [ ] Collect Prometheus metrics for all running applications

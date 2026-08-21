# Proxy to the Telegram Bot API

[Русская версия](proxy.md)

The bot only talks to `api.telegram.org` (plain HTTPS, long polling). If that
endpoint is unreachable from your server, there are two options: a proxy in
the bot config, or routing Telegram's address space through an existing
tunnel.

## Option 1: `telegram_proxies` in the config

```toml
# /etc/awgram/config.toml
telegram_proxies = [
  "socks5h://user:pass@10.8.0.1:1080",
  "http://10.8.0.2:3128",
]
```

- **Schemes**: `socks5://`, `socks5h://`, `http://`, `https://`.
  Credentials go directly in the URL; special characters in them must be
  percent-encoded (e.g. `p@ss` → `p%40ss`). Other schemes (including
  MTProto proxies) are not supported: the Bot API is plain HTTPS, MTProto
  does not apply to it.
- **Prefer `socks5h://`**: the `api.telegram.org` hostname is resolved on
  the proxy side. With `socks5://` DNS resolution happens locally — a weak
  spot when DNS responses are tampered with.
- **The list is a priority list, not round-robin**: at startup the bot
  probes the proxies in order with a lightweight `getMe` request (10 s
  timeout) and uses the first one that responds. If none respond, the bot
  exits with an error, systemd restarts it after 5 s, and probing starts
  over.
- **Runtime failover**: every 60 s the bot checks the connection; after
  three consecutive failures the process exits, systemd brings it back up —
  and selection starts again from the first proxy in the list. Worst case,
  a dead proxy takes ≈3×(60 + 10) ≈ 3.5 minutes to trigger a restart.
  While the proxy is dead the bot stays silent; the VPN itself keeps
  working as usual.
- **Logs**: proxy credentials never reach the logs or Debug output — only
  `scheme://***@host:port` is printed.

## Option 2: routing without a proxy

If the server already sends traffic through a foreign machine (e.g. clients
are routed via a tunnel), you don't need a proxy: route local traffic to
Telegram's addresses into that same tunnel — via policy routing or firewall
rules.

- Telegram's address space is stable; it is easy to obtain via **ASN62041**
  (e.g. from BGP data: `whois -h whois.radb.net -- '-i origin AS62041'`)
  and persist however you prefer.
- Don't forget **IPv6**: if the server has an IPv6 address and you only
  route the IPv4 prefixes, traffic will go out directly via the AAAA record.
- If the tunnel goes down the bot goes silent just like with a dead proxy —
  but there is no automatic failover here: restoring the tunnel is on you.

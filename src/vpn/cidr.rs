//! IPv4-арифметика для пресетов AllowedIPs: режим «весь трафик, кроме
//! выбранных сетей» требует выразить `0.0.0.0/0 − сети` списком CIDR — WireGuard
//! не умеет исключений, только перечисление. Без внешних зависимостей: нужны
//! разбор, вычитание и склейка соседних блоков, всё на u32.

use std::fmt;

/// Сеть IPv4: адрес уже приведён к границе префикса (host-биты обнулены).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ipv4Net {
    addr: u32,
    len: u8,
}

fn mask(len: u8) -> u32 {
    if len == 0 {
        0
    } else {
        u32::MAX << (32 - u32::from(len))
    }
}

impl Ipv4Net {
    pub fn new(addr: u32, len: u8) -> Option<Ipv4Net> {
        if len > 32 {
            return None;
        }
        Some(Ipv4Net {
            addr: addr & mask(len),
            len,
        })
    }

    /// `a.b.c.d/len`; host-биты обнуляются. Без `/len`, IPv6 и len > 32 — None.
    pub fn parse(s: &str) -> Option<Ipv4Net> {
        let (ip, len) = s.trim().split_once('/')?;
        let ip: std::net::Ipv4Addr = ip.trim().parse().ok()?;
        let len: u8 = len.trim().parse().ok()?;
        Ipv4Net::new(u32::from(ip), len)
    }

    pub fn contains(self, other: Ipv4Net) -> bool {
        self.len <= other.len && (other.addr & mask(self.len)) == self.addr
    }

    pub fn overlaps(self, other: Ipv4Net) -> bool {
        self.contains(other) || other.contains(self)
    }

    /// Две половины сети (`/len+1`). Для /32 половин нет.
    fn halves(self) -> Option<(Ipv4Net, Ipv4Net)> {
        if self.len >= 32 {
            return None;
        }
        let len = self.len + 1;
        let lo = Ipv4Net {
            addr: self.addr,
            len,
        };
        let hi = Ipv4Net {
            addr: self.addr | (1u32 << (32 - u32::from(len))),
            len,
        };
        Some((lo, hi))
    }

    /// Родительская сеть (`/len-1`), если этот блок — её половина.
    fn parent(self) -> Option<Ipv4Net> {
        if self.len == 0 {
            return None;
        }
        Ipv4Net::new(self.addr, self.len - 1)
    }
}

impl fmt::Display for Ipv4Net {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", std::net::Ipv4Addr::from(self.addr), self.len)
    }
}

/// Канонический список: без дублей и вложенных сетей, соседние блоки склеены,
/// отсортирован по адресу.
pub fn aggregate(nets: &[Ipv4Net]) -> Vec<Ipv4Net> {
    let mut sorted: Vec<Ipv4Net> = nets.to_vec();
    // По адресу, при равенстве — более широкая сеть первой: тогда вложенные
    // всегда идут после своего контейнера и отсекаются одним проходом.
    sorted.sort_by_key(|x| (x.addr, x.len));
    let mut out: Vec<Ipv4Net> = Vec::with_capacity(sorted.len());
    for net in sorted {
        if let Some(last) = out.last() {
            if last.contains(net) {
                continue;
            }
        }
        out.push(net);
        // Склейка sibling-пар: две половины одного родителя рядом → родитель;
        // повторяем, пока склейка тянется вверх.
        while out.len() >= 2 {
            let hi = out[out.len() - 1];
            let lo = out[out.len() - 2];
            match (lo.parent(), hi.parent()) {
                (Some(p), Some(q)) if p == q && lo.len == hi.len && lo.addr < hi.addr => {
                    out.truncate(out.len() - 2);
                    out.push(p);
                }
                _ => break,
            }
        }
    }
    out
}

fn subtract_one(base: Ipv4Net, cut: Ipv4Net, out: &mut Vec<Ipv4Net>) {
    if !base.overlaps(cut) {
        out.push(base);
        return;
    }
    if cut.contains(base) {
        return;
    }
    // cut строго внутри base: делим пополам и режем ту половину, где он лежит.
    let (lo, hi) = base
        .halves()
        .expect("cut inside base implies base is wider than /32");
    subtract_one(lo, cut, out);
    subtract_one(hi, cut, out);
}

/// `base − cut`, канонический список.
pub fn subtract(base: &[Ipv4Net], cut: &[Ipv4Net]) -> Vec<Ipv4Net> {
    let mut acc = aggregate(base);
    for c in aggregate(cut) {
        let mut next = Vec::with_capacity(acc.len() + 32);
        for b in acc {
            subtract_one(b, c, &mut next);
        }
        acc = next;
    }
    aggregate(&acc)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> Ipv4Net {
        Ipv4Net::parse(s).unwrap()
    }

    fn strs(v: &[Ipv4Net]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn parse_normalises_host_bits_and_roundtrips() {
        assert_eq!(n("10.1.2.3/8").to_string(), "10.0.0.0/8");
        assert_eq!(n("192.168.1.0/24").to_string(), "192.168.1.0/24");
        assert_eq!(n("0.0.0.0/0").to_string(), "0.0.0.0/0");
        assert_eq!(n("1.2.3.4/32").to_string(), "1.2.3.4/32");
    }

    #[test]
    fn parse_rejects_bare_ip_ipv6_and_bad_len() {
        for bad in ["1.2.3.4", "::/0", "10.0.0.0/33", "10.0.0/8", "x/8", ""] {
            assert!(Ipv4Net::parse(bad).is_none(), "should reject {bad:?}");
        }
    }

    #[test]
    fn contains_and_overlaps() {
        assert!(n("10.0.0.0/8").contains(n("10.9.9.0/24")));
        assert!(!n("10.9.9.0/24").contains(n("10.0.0.0/8")));
        assert!(n("10.0.0.0/8").contains(n("10.0.0.0/8")));
        assert!(n("10.0.0.0/8").overlaps(n("10.9.9.0/24")));
        assert!(n("10.9.9.0/24").overlaps(n("10.0.0.0/8")));
        assert!(!n("10.0.0.0/8").overlaps(n("192.168.0.0/16")));
    }

    #[test]
    fn subtract_all_minus_net10_matches_wireguard_list() {
        let got = subtract(&[n("0.0.0.0/0")], &[n("10.0.0.0/8")]);
        assert_eq!(
            strs(&got),
            [
                "0.0.0.0/5",
                "8.0.0.0/7",
                "11.0.0.0/8",
                "12.0.0.0/6",
                "16.0.0.0/4",
                "32.0.0.0/3",
                "64.0.0.0/2",
                "128.0.0.0/1",
            ]
        );
    }

    #[test]
    fn subtract_disjoint_keeps_base_and_superset_empties_it() {
        assert_eq!(
            strs(&subtract(&[n("10.0.0.0/8")], &[n("192.168.0.0/16")])),
            ["10.0.0.0/8"]
        );
        assert!(subtract(&[n("10.9.9.0/24")], &[n("10.0.0.0/8")]).is_empty());
    }

    #[test]
    fn subtract_is_idempotent() {
        let cut = [n("10.0.0.0/8"), n("192.168.1.0/24")];
        let once = subtract(&[n("0.0.0.0/0")], &cut);
        let twice = subtract(&once, &cut);
        assert_eq!(once, twice);
    }

    #[test]
    fn aggregate_merges_siblings_drops_nested_and_dedupes() {
        let got = aggregate(&[
            n("10.128.0.0/9"),
            n("10.0.0.0/9"),
            n("10.5.0.0/16"),
            n("192.168.0.0/16"),
            n("192.168.0.0/16"),
        ]);
        assert_eq!(strs(&got), ["10.0.0.0/8", "192.168.0.0/16"]);
    }

    #[test]
    fn aggregate_does_not_merge_misaligned_neighbours() {
        // 10.1/16 и 10.2/16 соседи, но не sibling-пара одного /15.
        let got = aggregate(&[n("10.1.0.0/16"), n("10.2.0.0/16")]);
        assert_eq!(strs(&got), ["10.1.0.0/16", "10.2.0.0/16"]);
    }

    #[test]
    fn readding_vpn_subnet_after_exclusion_keeps_it_routed() {
        let mut nets = subtract(&[n("0.0.0.0/0")], &[n("10.0.0.0/8")]);
        nets.push(n("10.9.9.0/24"));
        let got = aggregate(&nets);
        assert!(got.contains(&n("10.9.9.0/24")));
        assert!(!got.iter().any(|x| x.contains(n("10.0.0.1/32"))));
    }
}

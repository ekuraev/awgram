use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ValidateError {
    #[error("имя должно содержать 1–32 символа: латиница, цифры, дефис, подчёркивание")]
    BadName,
    #[error("срок должен быть в формате Nh/Nd/Nw, например 12h, 10d, 3w")]
    BadExpiry,
}

fn name_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Note: deviates from the brief's literal `^[A-Za-z0-9_-]{1,32}$` by forbidding a
    // leading hyphen. The literal pattern allows a hyphen anywhere, including first
    // position, so "--flag" would validate as a name yet be interpretable as a CLI
    // flag by the downstream script (argument injection). The brief's own test
    // `rejects_injection_and_bad_names` requires "--flag" to be rejected, so the
    // first character is restricted to alnum/underscore while the overall charset
    // and 1-32 length bound are unchanged.
    RE.get_or_init(|| Regex::new(r"^[A-Za-z0-9_][A-Za-z0-9_-]{0,31}$").unwrap())
}

fn expiry_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[0-9]{1,4}[hdw]$").unwrap())
}

pub fn validate_name(input: &str) -> Result<String, ValidateError> {
    let name = input.trim();
    if name_re().is_match(name) {
        Ok(name.to_string())
    } else {
        Err(ValidateError::BadName)
    }
}

/// Нормализация имени из диалога добавления: trim, каждая последовательность
/// пробельных символов → один дефис, опциональный слаг-префикс `{slug}-`,
/// затем та же валидация, что и в `validate_name`. Слишком длинный итог —
/// ошибка, а не молчаливая обрезка.
pub fn normalize_name(input: &str, slug: Option<&str>) -> Result<String, ValidateError> {
    let dashed = input.split_whitespace().collect::<Vec<_>>().join("-");
    if dashed.is_empty() {
        return Err(ValidateError::BadName);
    }
    let name = match slug {
        Some(s) => format!("{s}-{dashed}"),
        None => dashed,
    };
    if name_re().is_match(&name) {
        Ok(name)
    } else {
        Err(ValidateError::BadName)
    }
}

/// 5 случайных символов a-z0-9 (~60 млн комбинаций); коллизии дополнительно
/// отсекает проверка дубликатов `vpn.exists` в диалоге добавления.
pub fn gen_slug() -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    (0..5)
        .map(|_| CHARS[rand::random_range(0..CHARS.len())] as char)
        .collect()
}

/// Верхний предел массовой генерации — максимум `sendMediaGroup` (Telegram:
/// 2–10 элементов одного типа). Больше нельзя выдать одним альбомом.
pub const MAX_BULK: u32 = 10;

/// Длина слага из `gen_slug` (5 символов a-z0-9).
const SLUG_LEN: usize = 5;

/// Ширина числового суффикса — всегда по `MAX_BULK` (2 знака), независимо от
/// count: повторные генерации с одним префиксом дают единообразные имена.
fn bulk_suffix_width() -> usize {
    MAX_BULK.to_string().len()
}

/// Максимальная длина префикса массовой генерации: 32 (лимит `name_re`) минус
/// суффикс `-NN` и, при включённом ID-префиксе, минус `slug-`.
pub fn max_bulk_prefix_len(slug_enabled: bool) -> usize {
    let mut max = 32 - (1 + bulk_suffix_width()); // "-NN"
    if slug_enabled {
        max -= SLUG_LEN + 1; // "slug-"
    }
    max
}

/// Проверка префикса на худший случай (count = MAX_BULK, slug при включённой
/// настройке) — для ранней валидации на первом шаге диалога, чтобы пользователь
/// не узнавал о слишком длинном префиксе только после выбора срока и PSK.
pub fn validate_bulk_prefix(prefix: &str, slug_enabled: bool) -> Result<(), ValidateError> {
    let slug_placeholder = "0".repeat(SLUG_LEN);
    let slug = slug_enabled.then_some(slug_placeholder.as_str());
    gen_bulk_names(prefix, MAX_BULK, slug).map(|_| ())
}

/// Генерирует `count` имён вида `prefix-NN` (без slug) или `slug-prefix-NN`
/// (со slug, slug первым — как в `normalize_name`). Нумерация zero-padded по
/// ширине `MAX_BULK` (2 знака: 01..10), чтобы лексикографическая сортировка
/// совпадала с числовой и имена разных генераций были единообразны.
///
/// Каждое имя проходит `name_re()` (≤32 символа). Слишком длинный префикс
/// (с учётом slug и суффикса) → `Err(BadName)` — без молчаливой обрезки.
pub fn gen_bulk_names(
    prefix: &str,
    count: u32,
    slug: Option<&str>,
) -> Result<Vec<String>, ValidateError> {
    if count == 0 {
        return Err(ValidateError::BadName);
    }
    let prefix = prefix.trim();
    // Префикс должен сам состоять из допустимых символов (без shell-метасимволов,
    // пробелов и т.п.) — иначе сгенерённые имена не пройдут name_re().
    if !prefix
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        || prefix.is_empty()
    {
        return Err(ValidateError::BadName);
    }
    let width = bulk_suffix_width();
    let mut out = Vec::with_capacity(count as usize);
    for i in 1..=count {
        let suffix = format!("{:0width$}", i, width = width);
        let name = match slug {
            Some(s) => format!("{s}-{prefix}-{suffix}"),
            None => format!("{prefix}-{suffix}"),
        };
        if !name_re().is_match(&name) {
            return Err(ValidateError::BadName);
        }
        out.push(name);
    }
    Ok(out)
}

pub fn validate_expiry(input: &str) -> Result<String, ValidateError> {
    let v = input.trim();
    if expiry_re().is_match(v) {
        Ok(v.to_string())
    } else {
        Err(ValidateError::BadExpiry)
    }
}

/// Параметры клиента, которые бот умеет менять через `manage modify`.
/// CLI-имена совпадают с ключами в клиентском .conf (PersistentKeepalive/DNS/AllowedIPs/Endpoint).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifyParam {
    Keepalive,
    Dns,
    AllowedIps,
    Endpoint,
}

impl ModifyParam {
    /// Короткое имя для `details` в журнале событий (не путать с CLI-именем
    /// из `modify_param_cli`, которое уходит в manage.sh).
    pub fn as_str(self) -> &'static str {
        match self {
            ModifyParam::Keepalive => "keepalive",
            ModifyParam::Dns => "dns",
            ModifyParam::AllowedIps => "allowedips",
            ModifyParam::Endpoint => "endpoint",
        }
    }
}

pub fn modify_param_cli(p: ModifyParam) -> &'static str {
    match p {
        ModifyParam::Keepalive => "PersistentKeepalive",
        ModifyParam::Dns => "DNS",
        ModifyParam::AllowedIps => "AllowedIPs",
        ModifyParam::Endpoint => "Endpoint",
    }
}

fn keepalive_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[0-9]{1,5}$").unwrap())
}

/// 0..=65535 секунд (0 = off). Диапазон выровнен с инсталлером v5.21.0
/// (manage.sh:1024 `value -gt 65535`). Буквы/знаки/вне диапазона → ошибка.
pub fn parse_keepalive(input: &str) -> Result<String, ValidateError> {
    let v = input.trim();
    if !keepalive_re().is_match(v) {
        return Err(ValidateError::BadExpiry);
    }
    match v.parse::<u32>() {
        Ok(n) if n <= 65535 => Ok(n.to_string()),
        _ => Err(ValidateError::BadExpiry),
    }
}

/// 1..=4 IP-адресов (v4/v6) через запятую. Shell-метасимволы невозможны —
/// `IpAddr::from_str` их не примет.
pub fn parse_dns(input: &str) -> Result<String, ValidateError> {
    let parts: Vec<&str> = input.split(',').map(|s| s.trim()).collect();
    if parts.is_empty() || parts.len() > 4 || parts.iter().any(|s| s.is_empty()) {
        return Err(ValidateError::BadExpiry);
    }
    for p in &parts {
        if p.parse::<std::net::IpAddr>().is_err() {
            return Err(ValidateError::BadExpiry);
        }
    }
    Ok(parts.join(", "))
}

fn cidr_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // IPv4 CIDR или IPv6 CIDR. Не принимаем ничего с shell-метасимволами: в
    // шаблон не входят ; | & $ ` < > и т.д.
    RE.get_or_init(|| {
        Regex::new(r"^(?:[0-9]{1,3}(?:\.[0-9]{1,3}){3}/[0-9]{1,2}|[0-9a-fA-F:]+/[0-9]{1,3})$")
            .unwrap()
    })
}

/// CIDR-список через запятую. Синтаксическая проверка; валидность подсети
/// оставляем скрипту.
pub fn parse_allowed_ips(input: &str) -> Result<String, ValidateError> {
    let parts: Vec<&str> = input.split(',').map(|s| s.trim()).collect();
    if parts.is_empty() || parts.iter().any(|s| s.is_empty()) {
        return Err(ValidateError::BadExpiry);
    }
    for p in &parts {
        if !cidr_re().is_match(p) {
            return Err(ValidateError::BadExpiry);
        }
    }
    Ok(parts.join(", "))
}

fn endpoint_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Две формы: host:port (host = FQDN/IPv4) или [IPv6]:port с ОБЯЗАТЕЛЬНЫМИ
    // парными скобками. Инсталлер требует именно [IPv6]:port — непарные скобки
    // или голый IPv6 с двоеточиями неотличимы от host:port и парсерятся бы
    // неверно. Запрещаем shell-метасимволы.
    RE.get_or_init(|| Regex::new(r"^(?:\[[0-9a-fA-F:.]+\]|[A-Za-z0-9._-]+):[0-9]{1,5}$").unwrap())
}

/// Endpoint в формате host:port или [IPv6]:port. Порт проверяется в диапазоне
/// 1..=65535 (инсталлер manage.sh:1034). Shell-метасимволы отсекаются regex.
pub fn parse_endpoint(input: &str) -> Result<String, ValidateError> {
    let v = input.trim();
    if !endpoint_re().is_match(v) {
        return Err(ValidateError::BadExpiry);
    }
    // Извлекаем порт: для [IPv6]:port — после ']'; для host:port — после ':'.
    let port_str = if v.contains(']') {
        // [IPv6]:port → берём часть после ']'
        v.rsplit_once(']')
            .map(|(_, rest)| rest.trim_start_matches(':'))
    } else {
        // host:port → после последнего ':'
        v.rsplit_once(':').map(|(_, port)| port)
    }
    .unwrap_or("");
    match port_str.parse::<u32>() {
        Ok(p) if (1..=65535).contains(&p) => Ok(v.to_string()),
        _ => Err(ValidateError::BadExpiry),
    }
}

pub fn parse_modify_value(p: ModifyParam, input: &str) -> Result<String, ValidateError> {
    match p {
        ModifyParam::Keepalive => parse_keepalive(input),
        ModifyParam::Dns => parse_dns(input),
        ModifyParam::AllowedIps => parse_allowed_ips(input),
        ModifyParam::Endpoint => parse_endpoint(input),
    }
}

// --- Пресеты AllowedIPs (маршруты клиента) ---
//
// Инсталлер задаёт AllowedIPs новому клиенту глобальным режимом сервера и не
// принимает их флагом у `add` (только `modify` постфактум) — поэтому бот
// собирает строку сам, из набора готовых подсетей, и применяет её отдельным
// вызовом `modify`. Здесь — только сборка/разбор строки: чистая логика,
// пригодная для юнит-тестов.
//
// Два режима экрана. «Направлять»: AllowedIPs = выбранные сети (+ подсеть
// VPN). «Исключать»: AllowedIPs = весь трафик минус выбранные сети — WireGuard
// не знает исключений, поэтому дополнение считается явно (см. `cidr`), а
// подсеть VPN всегда возвращается в список: исключение 10.0.0.0/8 иначе
// отрезало бы сам туннель.

use crate::vpn::cidr::{self, Ipv4Net};

/// Приватные диапазоны RFC 1918 — «локальные сети» в терминах экрана выбора.
pub const NET_10: &str = "10.0.0.0/8";
pub const NET_172: &str = "172.16.0.0/12";
pub const NET_192: &str = "192.168.0.0/16";
/// Полный туннель: весь IPv4 и весь IPv6.
pub const ROUTE_ALL: &str = "0.0.0.0/0, ::/0";

/// Сетевые пресеты экрана: три диапазона RFC 1918 целиком и типовые /24
/// домашних роутеров. Порядок — порядок в собранной строке и на клавиатуре.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetPreset {
    Net10,
    Net172,
    Net192,
    Net10_0,
    Net10_1,
    Net192_0,
    Net192_1,
    Net192_10,
    Net192_100,
}

impl NetPreset {
    pub const ALL: [NetPreset; 9] = [
        NetPreset::Net10,
        NetPreset::Net172,
        NetPreset::Net192,
        NetPreset::Net10_0,
        NetPreset::Net10_1,
        NetPreset::Net192_0,
        NetPreset::Net192_1,
        NetPreset::Net192_10,
        NetPreset::Net192_100,
    ];
    /// Три диапазона RFC 1918 — состав группового тумблера «все локальные».
    pub const WIDE: [NetPreset; 3] = [NetPreset::Net10, NetPreset::Net172, NetPreset::Net192];

    pub fn cidr(self) -> &'static str {
        match self {
            NetPreset::Net10 => NET_10,
            NetPreset::Net172 => NET_172,
            NetPreset::Net192 => NET_192,
            NetPreset::Net10_0 => "10.0.0.0/24",
            NetPreset::Net10_1 => "10.0.1.0/24",
            NetPreset::Net192_0 => "192.168.0.0/24",
            NetPreset::Net192_1 => "192.168.1.0/24",
            NetPreset::Net192_10 => "192.168.10.0/24",
            NetPreset::Net192_100 => "192.168.100.0/24",
        }
    }

    /// Ключ в callback-data (`aip:t:<key>`); «10»/«172»/«192» — исторические.
    pub fn key(self) -> &'static str {
        match self {
            NetPreset::Net10 => "10",
            NetPreset::Net172 => "172",
            NetPreset::Net192 => "192",
            NetPreset::Net10_0 => "10.0",
            NetPreset::Net10_1 => "10.1",
            NetPreset::Net192_0 => "192.0",
            NetPreset::Net192_1 => "192.1",
            NetPreset::Net192_10 => "192.10",
            NetPreset::Net192_100 => "192.100",
        }
    }

    fn from_key(s: &str) -> Option<NetPreset> {
        NetPreset::ALL.into_iter().find(|p| p.key() == s)
    }

    fn from_cidr(s: &str) -> Option<NetPreset> {
        NetPreset::ALL.into_iter().find(|p| p.cidr() == s)
    }

    fn net(self) -> Ipv4Net {
        Ipv4Net::parse(self.cidr()).expect("preset CIDRs are valid")
    }

    fn bit(self) -> u16 {
        1 << (NetPreset::ALL
            .iter()
            .position(|p| *p == self)
            .expect("preset in ALL"))
    }
}

/// Кнопки экрана выбора маршрутов (одна на пресет).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteKey {
    All,
    Vpn,
    /// Групповой тумблер «все локальные сети» — три диапазона RFC 1918 разом.
    Local,
    Net(NetPreset),
}

impl RouteKey {
    pub fn as_str(self) -> &'static str {
        match self {
            RouteKey::All => "all",
            RouteKey::Vpn => "vpn",
            RouteKey::Local => "local",
            RouteKey::Net(p) => p.key(),
        }
    }

    pub fn parse_str(s: &str) -> Option<RouteKey> {
        match s {
            "all" => Some(RouteKey::All),
            "vpn" => Some(RouteKey::Vpn),
            "local" => Some(RouteKey::Local),
            _ => NetPreset::from_key(s).map(RouteKey::Net),
        }
    }
}

/// Режим экрана: выбранные сети идут в туннель или мимо него.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RouteMode {
    #[default]
    Include,
    Exclude,
}

/// Набор включённых пресетов. `all` эксклюзивен: 0.0.0.0/0 поглощает любую
/// подсеть, поэтому его включение гасит остальные тумблеры (и наоборот) —
/// иначе экран показывал бы взаимоисключающие галки. В режиме исключений
/// `all` и `vpn` смысла не имеют и всегда сброшены.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RouteSelection {
    pub mode: RouteMode,
    pub all: bool,
    pub vpn: bool,
    nets: u16,
}

impl RouteSelection {
    /// Набор из перечня сетевых пресетов (для тестов и предзаполнения).
    pub fn with_nets(mode: RouteMode, nets: &[NetPreset]) -> RouteSelection {
        let mut sel = RouteSelection {
            mode,
            ..RouteSelection::default()
        };
        for p in nets {
            sel.set_net(*p, true);
        }
        sel
    }

    /// «Весь трафик»: единственный тумблер, остальные сброшены.
    pub fn all_traffic() -> RouteSelection {
        RouteSelection {
            all: true,
            ..RouteSelection::default()
        }
    }

    pub fn is_empty(self) -> bool {
        !(self.all || self.vpn || self.nets != 0)
    }

    pub fn net(self, p: NetPreset) -> bool {
        self.nets & p.bit() != 0
    }

    pub fn set_net(&mut self, p: NetPreset, on: bool) {
        if on {
            self.nets |= p.bit();
        } else {
            self.nets &= !p.bit();
        }
    }

    /// Выбранные сетевые пресеты в каноническом порядке.
    pub fn nets(self) -> impl Iterator<Item = NetPreset> {
        NetPreset::ALL.into_iter().filter(move |p| self.net(*p))
    }

    /// Все три диапазона RFC 1918 включены (состояние группового тумблера).
    pub fn local_all(self) -> bool {
        NetPreset::WIDE.iter().all(|p| self.net(*p))
    }

    pub fn get(self, key: RouteKey) -> bool {
        match key {
            RouteKey::All => self.all,
            RouteKey::Vpn => self.vpn,
            RouteKey::Local => self.local_all(),
            RouteKey::Net(p) => self.net(p),
        }
    }

    /// Переключает пресет с учётом эксклюзивности «весь трафик». В режиме
    /// исключений «весь трафик» и «сеть VPN» на экране отсутствуют, но
    /// устаревший callback их всё же может прислать — тогда возвращаемся
    /// в режим «направлять», где они имеют смысл.
    pub fn toggle(&mut self, key: RouteKey) {
        match key {
            RouteKey::All => {
                let on = !self.all;
                *self = RouteSelection {
                    all: on,
                    ..RouteSelection::default()
                };
            }
            RouteKey::Vpn => {
                self.mode = RouteMode::Include;
                self.all = false;
                self.vpn = !self.vpn;
            }
            RouteKey::Local => {
                let on = !self.local_all();
                self.all = false;
                for p in NetPreset::WIDE {
                    self.set_net(p, on);
                }
            }
            RouteKey::Net(p) => {
                self.all = false;
                self.set_net(p, !self.net(p));
            }
        }
    }

    /// Смена режима: сетевые тумблеры сохраняются, «весь трафик» и «сеть VPN»
    /// сбрасываются — в режиме исключений их нет.
    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            RouteMode::Include => RouteMode::Exclude,
            RouteMode::Exclude => RouteMode::Include,
        };
        self.all = false;
        self.vpn = false;
    }
}

/// Собирает значение AllowedIPs из набора тумблеров. `None` — ничего не
/// выбрано (применять нечего). Подсеть VPN участвует только если она известна
/// (её отдаёт `check`, при недоступном интерфейсе кнопки просто нет).
pub fn build_allowed_ips(sel: RouteSelection, vpn_subnet: Option<&str>) -> Option<String> {
    match sel.mode {
        RouteMode::Include => build_include(sel, vpn_subnet),
        RouteMode::Exclude => build_exclude(sel, vpn_subnet),
    }
}

fn build_include(sel: RouteSelection, vpn_subnet: Option<&str>) -> Option<String> {
    if sel.all {
        return Some(ROUTE_ALL.to_string());
    }
    let mut parts: Vec<&str> = sel.nets().map(NetPreset::cidr).collect();
    if let (true, Some(subnet)) = (sel.vpn, vpn_subnet) {
        parts.push(subnet);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn build_exclude(sel: RouteSelection, vpn_subnet: Option<&str>) -> Option<String> {
    let cut: Vec<Ipv4Net> = sel.nets().map(NetPreset::net).collect();
    if cut.is_empty() {
        return None;
    }
    let everything = Ipv4Net::parse("0.0.0.0/0").expect("literal");
    let mut nets = cidr::subtract(&[everything], &cut);
    if let Some(vpn) = vpn_subnet.and_then(Ipv4Net::parse) {
        nets.push(vpn);
        nets = cidr::aggregate(&nets);
    }
    let mut parts: Vec<String> = nets.iter().map(ToString::to_string).collect();
    parts.push("::/0".to_string());
    Some(parts.join(", "))
}

/// Разбирает текущее значение AllowedIPs в набор тумблеров. `None` — значение
/// не выражается пресетами (задано вручную): экран покажет его как есть и
/// оставит тумблеры пустыми, чтобы не подменять чужую настройку.
pub fn selection_from_value(value: &str, vpn_subnet: Option<&str>) -> Option<RouteSelection> {
    selection_from_include(value, vpn_subnet).or_else(|| selection_from_exclude(value, vpn_subnet))
}

fn selection_from_include(value: &str, vpn_subnet: Option<&str>) -> Option<RouteSelection> {
    let mut sel = RouteSelection::default();
    let mut seen = false;
    for raw in value.split(',') {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        seen = true;
        match token {
            "0.0.0.0/0" | "::/0" => sel.all = true,
            t if Some(t) == vpn_subnet => sel.vpn = true,
            t => sel.set_net(NetPreset::from_cidr(t)?, true),
        }
    }
    // «Весь трафик» несовместим с подсетями: такое значение собрано не нами.
    if sel.all && (sel.nets != 0 || sel.vpn) {
        return None;
    }
    if seen {
        Some(sel)
    } else {
        None
    }
}

/// Значение вида «всё, кроме…»: пресет считается исключённым, если в списке
/// нет ничего из него (за вычетом подсети VPN — её сборка возвращает всегда).
/// Вложенные исключения (192.168.1.0/24 внутри 192.168.0.0/16) сворачиваются
/// в широкий тумблер. Итог сверяется пересборкой: не совпало — значение чужое.
fn selection_from_exclude(value: &str, vpn_subnet: Option<&str>) -> Option<RouteSelection> {
    let mut have_v6 = false;
    let mut v4: Vec<Ipv4Net> = Vec::new();
    for raw in value.split(',') {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        if token == "::/0" {
            have_v6 = true;
            continue;
        }
        v4.push(Ipv4Net::parse(token)?);
    }
    if !have_v6 || v4.is_empty() {
        return None;
    }
    let vpn: Vec<Ipv4Net> = vpn_subnet.and_then(Ipv4Net::parse).into_iter().collect();
    let excluded: Vec<NetPreset> = NetPreset::ALL
        .into_iter()
        .filter(|p| {
            let body = cidr::subtract(&[p.net()], &vpn);
            !body.is_empty() && !body.iter().any(|b| v4.iter().any(|v| b.overlaps(*v)))
        })
        .collect();
    let outer: Vec<NetPreset> = excluded
        .iter()
        .copied()
        .filter(|p| !excluded.iter().any(|q| q != p && q.net().contains(p.net())))
        .collect();
    let sel = RouteSelection::with_nets(RouteMode::Exclude, &outer);
    let rebuilt = build_allowed_ips(sel, vpn_subnet)?;
    let rebuilt_v4: Vec<Ipv4Net> = rebuilt
        .split(',')
        .map(str::trim)
        .filter_map(Ipv4Net::parse)
        .collect();
    if cidr::aggregate(&v4) == rebuilt_v4 {
        Some(sel)
    } else {
        None
    }
}

/// Адрес интерфейса (`10.9.9.1/24`) → адрес его сети (`10.9.9.0/24`).
/// Только IPv4: подсеть VPN нужна как маршрут, а v6-префикс /64 в пресетах
/// не участвует.
pub fn network_cidr(addr: &str) -> Option<String> {
    let (ip, len) = addr.split_once('/')?;
    let len: u32 = len.trim().parse().ok()?;
    if len > 32 {
        return None;
    }
    let ip: std::net::Ipv4Addr = ip.trim().parse().ok()?;
    let mask = if len == 0 { 0 } else { u32::MAX << (32 - len) };
    let net = std::net::Ipv4Addr::from(u32::from(ip) & mask);
    Some(format!("{net}/{len}"))
}

/// Подсеть VPN из `interface.addresses` отчёта `check`: первый IPv4-адрес,
/// приведённый к адресу сети. Нет IPv4 — нет и пресета.
pub fn vpn_subnet_from_addresses(addresses: &[String]) -> Option<String> {
    addresses
        .iter()
        .find(|a| !a.contains(':'))
        .and_then(|a| network_cidr(a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_good_names() {
        assert_eq!(validate_name("alice").unwrap(), "alice");
        assert_eq!(validate_name("  bob_1-2  ").unwrap(), "bob_1-2");
    }

    #[test]
    fn rejects_injection_and_bad_names() {
        for bad in [
            "",
            "a b",
            "a;rm -rf /",
            "../etc",
            "имя",
            "a".repeat(33).as_str(),
            "--flag",
            "a/b",
        ] {
            assert_eq!(
                validate_name(bad),
                Err(ValidateError::BadName),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn accepts_good_expiry() {
        for good in ["12h", "10d", "3w", "1d", "9999h"] {
            assert!(validate_expiry(good).is_ok(), "should accept {good}");
        }
    }

    #[test]
    fn rejects_bad_expiry() {
        for bad in ["", "10", "d10", "10x", "1.5d", "10 d", "-5d", "10d;ls"] {
            assert_eq!(
                validate_expiry(bad),
                Err(ValidateError::BadExpiry),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn normalize_replaces_spaces_with_dashes() {
        assert_eq!(normalize_name("work laptop", None).unwrap(), "work-laptop");
        assert_eq!(
            normalize_name("work   laptop", None).unwrap(),
            "work-laptop"
        );
        assert_eq!(normalize_name("  alice  ", None).unwrap(), "alice");
    }

    #[test]
    fn normalize_adds_slug_prefix() {
        assert_eq!(
            normalize_name("alice", Some("k3x9f")).unwrap(),
            "k3x9f-alice"
        );
        assert_eq!(
            normalize_name("work laptop", Some("k3x9f")).unwrap(),
            "k3x9f-work-laptop"
        );
    }

    #[test]
    fn normalize_rejects_empty_and_whitespace_only() {
        assert_eq!(normalize_name("", None), Err(ValidateError::BadName));
        assert_eq!(normalize_name("   ", None), Err(ValidateError::BadName));
        // с включённым слагом пустое имя тоже отклоняется, а не превращается в "k3x9f-"
        assert_eq!(
            normalize_name("   ", Some("k3x9f")),
            Err(ValidateError::BadName)
        );
    }

    #[test]
    fn normalize_rejects_too_long_with_slug() {
        let name26 = "a".repeat(26);
        assert!(normalize_name(&name26, Some("k3x9f")).is_ok()); // 5+1+26 = 32
        let name27 = "a".repeat(27);
        assert_eq!(
            normalize_name(&name27, Some("k3x9f")),
            Err(ValidateError::BadName)
        );
    }

    #[test]
    fn normalize_still_rejects_injection() {
        for bad in ["a;rm -rf /", "../etc", "имя", "--flag"] {
            assert_eq!(
                normalize_name(bad, None),
                Err(ValidateError::BadName),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn normalize_slug_makes_leading_dash_safe() {
        // без слага "--flag" отклоняется правилом первого символа; со слагом
        // первый символ — из слага, ведущего дефиса нет, инъекция CLI-флага невозможна
        assert_eq!(
            normalize_name("--flag", Some("k3x9f")).unwrap(),
            "k3x9f---flag"
        );
    }

    #[test]
    fn gen_slug_is_5_base36_chars() {
        for _ in 0..100 {
            let s = gen_slug();
            assert_eq!(s.len(), 5);
            assert!(
                s.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()),
                "bad slug {s:?}"
            );
        }
    }

    #[test]
    fn keepalive_accepts_valid_range() {
        // P2.5: инсталлер принимает 0..=65535 (manage.sh:1024), не 0..=600.
        assert_eq!(parse_keepalive("0").unwrap(), "0");
        assert_eq!(parse_keepalive("25").unwrap(), "25");
        assert_eq!(parse_keepalive("65535").unwrap(), "65535");
    }

    #[test]
    fn keepalive_rejects_out_of_range_and_non_numeric() {
        for bad in ["", "abc", "-1", "65536", "99999", "1.5", "25s"] {
            assert_eq!(
                parse_keepalive(bad),
                Err(ValidateError::BadExpiry),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn dns_accepts_ip_list() {
        assert_eq!(parse_dns("1.1.1.1").unwrap(), "1.1.1.1");
        assert_eq!(parse_dns("1.1.1.1, 8.8.8.8").unwrap(), "1.1.1.1, 8.8.8.8");
        assert!(parse_dns("2606:4700:4700::1111").is_ok());
    }

    #[test]
    fn dns_rejects_non_ip_and_too_many() {
        for bad in [
            "",
            "not-ip",
            "1.1.1.1; rm -rf /",
            "a.b.c.d",
            "1.1.1.1,",
            "8.8.8.8 1.1.1.1",
        ] {
            assert_eq!(
                parse_dns(bad),
                Err(ValidateError::BadExpiry),
                "should reject {bad:?}"
            );
        }
        // > 4 адресов
        let five = "1.1.1.1, 2.2.2.2, 3.3.3.3, 4.4.4.4, 5.5.5.5";
        assert_eq!(parse_dns(five), Err(ValidateError::BadExpiry));
    }

    #[test]
    fn allowed_ips_accepts_cidr() {
        assert!(parse_allowed_ips("0.0.0.0/0").is_ok());
        assert!(parse_allowed_ips("192.168.1.0/24, 10.0.0.0/8").is_ok());
        assert!(parse_allowed_ips("::/0").is_ok());
    }

    #[test]
    fn allowed_ips_rejects_non_cidr_and_shell_meta() {
        for bad in ["", "192.168.1.5", "not-cidr", "1.1.1.1; ls", "../etc"] {
            assert_eq!(
                parse_allowed_ips(bad),
                Err(ValidateError::BadExpiry),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn endpoint_accepts_host_port() {
        assert!(parse_endpoint("vpn.example.com:51820").is_ok());
        assert!(parse_endpoint("1.2.3.4:51820").is_ok());
        assert!(parse_endpoint("[2606:4700::1]:51820").is_ok());
        assert!(parse_endpoint("host:1").is_ok());
        assert!(parse_endpoint("host:65535").is_ok());
    }

    #[test]
    fn endpoint_rejects_missing_port_and_meta() {
        for bad in ["vpn.example.com", "", ":51820", "a.b:51820; rm", "host:abc"] {
            assert_eq!(
                parse_endpoint(bad),
                Err(ValidateError::BadExpiry),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn endpoint_rejects_port_out_of_range() {
        // P2.4: инсталлер требует порт 1..=65535 (manage.sh:1034).
        for bad in [
            "host:0",
            "host:65536",
            "host:99999",
            "1.2.3.4:0",
            "[::1]:99999",
        ] {
            assert_eq!(
                parse_endpoint(bad),
                Err(ValidateError::BadExpiry),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn endpoint_rejects_unpaired_ipv6_brackets() {
        // P2.4: инсталлер требует [IPv6]:port с парными скобками.
        for bad in [
            "[::1:51820",
            "::1]:51820",
            "[::1]51820",
            "2606:4700::1:51820",
        ] {
            assert_eq!(
                parse_endpoint(bad),
                Err(ValidateError::BadExpiry),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn modify_param_cli_names() {
        assert_eq!(
            modify_param_cli(ModifyParam::Keepalive),
            "PersistentKeepalive"
        );
        assert_eq!(modify_param_cli(ModifyParam::Dns), "DNS");
        assert_eq!(modify_param_cli(ModifyParam::AllowedIps), "AllowedIPs");
        assert_eq!(modify_param_cli(ModifyParam::Endpoint), "Endpoint");
    }

    #[test]
    fn modify_param_as_str_names() {
        assert_eq!(ModifyParam::Keepalive.as_str(), "keepalive");
        assert_eq!(ModifyParam::Dns.as_str(), "dns");
        assert_eq!(ModifyParam::AllowedIps.as_str(), "allowedips");
        assert_eq!(ModifyParam::Endpoint.as_str(), "endpoint");
    }

    #[test]
    fn parse_modify_value_dispatches_by_param() {
        assert!(parse_modify_value(ModifyParam::Keepalive, "25").is_ok());
        assert!(parse_modify_value(ModifyParam::Dns, "1.1.1.1").is_ok());
        assert!(parse_modify_value(ModifyParam::Keepalive, "abc").is_err());
    }

    #[test]
    fn gen_bulk_names_zero_pads_by_width() {
        let names = gen_bulk_names("user", 10, None).unwrap();
        assert_eq!(names.len(), 10);
        assert_eq!(names[0], "user-01");
        assert_eq!(names[9], "user-10");
    }

    #[test]
    fn gen_bulk_names_small_count_pads_to_max_bulk_width() {
        // Ширина суффикса всегда по MAX_BULK (=2), а не по count: повторные
        // генерации с одним префиксом дают единообразные имена (user-01,
        // а не user-1 vs user-01) и одинаковую сортировку.
        let names = gen_bulk_names("user", 3, None).unwrap();
        assert_eq!(names, vec!["user-01", "user-02", "user-03"]);
    }

    #[test]
    fn gen_bulk_names_with_slug_prefix_first() {
        let names = gen_bulk_names("user", 2, Some("k3x9f")).unwrap();
        assert_eq!(names, vec!["k3x9f-user-01", "k3x9f-user-02"]);
    }

    #[test]
    fn max_bulk_prefix_len_accounts_for_slug_and_suffix() {
        // 32 − "-NN"(3) = 29 без slug; минус "k3x9f-"(6) = 23 со slug.
        assert_eq!(max_bulk_prefix_len(false), 29);
        assert_eq!(max_bulk_prefix_len(true), 23);
    }

    #[test]
    fn validate_bulk_prefix_checks_worst_case_length() {
        // Граница без slug: 29 ок, 30 — уже нет (29+3 = 32, 30+3 = 33).
        assert!(validate_bulk_prefix(&"a".repeat(29), false).is_ok());
        assert!(validate_bulk_prefix(&"a".repeat(30), false).is_err());
        // Со slug граница сдвигается: 23 ок, 24 — нет.
        assert!(validate_bulk_prefix(&"a".repeat(23), true).is_ok());
        assert!(validate_bulk_prefix(&"a".repeat(24), true).is_err());
    }

    #[test]
    fn validate_bulk_prefix_rejects_bad_charset() {
        assert!(validate_bulk_prefix("user;rm", false).is_err());
        assert!(validate_bulk_prefix("", false).is_err());
    }

    #[test]
    fn gen_bulk_names_rejects_too_long_prefix() {
        // slug(5) + "-" + prefix(27) + "-NN" = 5+1+27+3 = 36 > 32
        let long = "a".repeat(27);
        assert_eq!(
            gen_bulk_names(&long, 2, Some("k3x9f")),
            Err(ValidateError::BadName)
        );
    }

    #[test]
    fn gen_bulk_names_rejects_zero_count() {
        assert!(gen_bulk_names("user", 0, None).is_err());
    }

    #[test]
    fn gen_bulk_names_rejects_injection_prefix() {
        // префикс с shell-метасимволами не должен проходить
        assert!(gen_bulk_names("user;rm", 2, None).is_err());
    }

    // --- пресеты AllowedIPs ---

    fn inc(nets: &[NetPreset]) -> RouteSelection {
        RouteSelection::with_nets(RouteMode::Include, nets)
    }

    fn exc(nets: &[NetPreset]) -> RouteSelection {
        RouteSelection::with_nets(RouteMode::Exclude, nets)
    }

    #[test]
    fn build_allowed_ips_joins_selected_nets_in_order() {
        let mut sel = inc(&[NetPreset::Net192, NetPreset::Net10]);
        sel.vpn = true;
        assert_eq!(
            build_allowed_ips(sel, Some("10.9.9.0/24")).unwrap(),
            "10.0.0.0/8, 192.168.0.0/16, 10.9.9.0/24"
        );
    }

    #[test]
    fn build_allowed_ips_narrow_presets_have_expected_cidrs() {
        let sel = inc(&[
            NetPreset::Net192_1,
            NetPreset::Net10_0,
            NetPreset::Net192_100,
        ]);
        assert_eq!(
            build_allowed_ips(sel, None).unwrap(),
            "10.0.0.0/24, 192.168.1.0/24, 192.168.100.0/24"
        );
    }

    #[test]
    fn build_allowed_ips_all_wins_and_empty_is_none() {
        let all = RouteSelection {
            all: true,
            ..RouteSelection::default()
        };
        assert_eq!(
            build_allowed_ips(all, Some("10.9.9.0/24")).unwrap(),
            ROUTE_ALL
        );
        assert!(build_allowed_ips(RouteSelection::default(), None).is_none());
    }

    #[test]
    fn build_allowed_ips_skips_vpn_when_subnet_unknown() {
        let sel = RouteSelection {
            vpn: true,
            ..RouteSelection::default()
        };
        assert!(build_allowed_ips(sel, None).is_none());
    }

    #[test]
    fn toggle_all_is_exclusive_both_ways() {
        let mut sel = inc(&[NetPreset::Net10]);
        sel.vpn = true;
        sel.toggle(RouteKey::All);
        assert_eq!(
            sel,
            RouteSelection {
                all: true,
                ..RouteSelection::default()
            }
        );
        // Любой другой тумблер снимает «весь трафик».
        sel.toggle(RouteKey::Net(NetPreset::Net172));
        assert!(!sel.all && sel.net(NetPreset::Net172));
    }

    #[test]
    fn toggle_local_sets_and_clears_all_three_rfc1918() {
        let mut sel = RouteSelection::default();
        sel.toggle(RouteKey::Local);
        assert!(sel.local_all());
        assert_eq!(sel.nets().collect::<Vec<_>>(), NetPreset::WIDE);
        sel.toggle(RouteKey::Local);
        assert!(sel.is_empty());
    }

    #[test]
    fn toggle_local_turns_on_when_only_part_selected() {
        // Частичный выбор — групповой тумблер добирает остальные, а не гасит.
        let mut sel = inc(&[NetPreset::Net10]);
        sel.toggle(RouteKey::Local);
        assert!(sel.local_all());
    }

    #[test]
    fn toggle_local_leaves_narrow_presets_alone() {
        let mut sel = inc(&[NetPreset::Net192_1]);
        sel.toggle(RouteKey::Local);
        assert!(sel.local_all() && sel.net(NetPreset::Net192_1));
    }

    #[test]
    fn route_key_roundtrips_all_presets() {
        for p in NetPreset::ALL {
            assert_eq!(RouteKey::parse_str(p.key()), Some(RouteKey::Net(p)));
        }
        assert_eq!(
            RouteKey::parse_str("10"),
            Some(RouteKey::Net(NetPreset::Net10))
        );
        assert_eq!(
            RouteKey::parse_str("192.1"),
            Some(RouteKey::Net(NetPreset::Net192_1))
        );
        assert_eq!(RouteKey::parse_str("local"), Some(RouteKey::Local));
        assert_eq!(RouteKey::parse_str("0.0.0.0/0"), None);
        assert_eq!(RouteKey::parse_str(""), None);
    }

    #[test]
    fn selection_from_value_roundtrips_presets() {
        let sel = selection_from_value("10.0.0.0/8, 172.16.0.0/12", None).unwrap();
        assert!(sel.net(NetPreset::Net10) && sel.net(NetPreset::Net172));
        assert!(!sel.net(NetPreset::Net192));
        assert_eq!(sel.mode, RouteMode::Include);
        assert_eq!(
            build_allowed_ips(sel, None).unwrap(),
            "10.0.0.0/8, 172.16.0.0/12"
        );
        let all = selection_from_value("0.0.0.0/0, ::/0", None).unwrap();
        assert!(all.all);
        let vpn = selection_from_value("10.9.9.0/24", Some("10.9.9.0/24")).unwrap();
        assert!(vpn.vpn);
        let narrow = selection_from_value("192.168.1.0/24", None).unwrap();
        assert!(narrow.net(NetPreset::Net192_1));
    }

    #[test]
    fn selection_from_value_none_for_manual_or_empty_values() {
        // Значение вне пресетов — не подменяем его тумблерами.
        assert!(selection_from_value("1.2.3.0/24", None).is_none());
        assert!(selection_from_value("10.9.9.0/24", None).is_none());
        assert!(selection_from_value("", None).is_none());
        // Полный туннель вперемешку с подсетью собран не нами.
        assert!(selection_from_value("0.0.0.0/0, 10.0.0.0/8", None).is_none());
    }

    #[test]
    fn build_allowed_ips_output_passes_parse_allowed_ips() {
        // Всё, что собирает экран, обязано пройти валидатор перед modify.
        let mut sel = inc(&NetPreset::ALL);
        sel.vpn = true;
        let v = build_allowed_ips(sel, Some("10.9.9.0/24")).unwrap();
        assert!(parse_allowed_ips(&v).is_ok());
        assert!(parse_allowed_ips(ROUTE_ALL).is_ok());
        let v = build_allowed_ips(exc(&NetPreset::ALL), Some("10.9.9.0/24")).unwrap();
        assert!(parse_allowed_ips(&v).is_ok());
    }

    // --- режим исключений ---

    #[test]
    fn exclude_net10_gives_complement_plus_ipv6() {
        assert_eq!(
            build_allowed_ips(exc(&[NetPreset::Net10]), None).unwrap(),
            "0.0.0.0/5, 8.0.0.0/7, 11.0.0.0/8, 12.0.0.0/6, 16.0.0.0/4, 32.0.0.0/3, 64.0.0.0/2, 128.0.0.0/1, ::/0"
        );
    }

    #[test]
    fn exclude_keeps_vpn_subnet_inside_excluded_range() {
        let v = build_allowed_ips(exc(&[NetPreset::Net10]), Some("10.9.9.0/24")).unwrap();
        assert!(v.contains("10.9.9.0/24"), "{v}");
        assert!(!v.contains("10.0.0.0/8"), "{v}");
        assert!(v.ends_with("::/0"), "{v}");
    }

    #[test]
    fn exclude_with_nothing_selected_is_none() {
        assert!(build_allowed_ips(exc(&[]), Some("10.9.9.0/24")).is_none());
    }

    #[test]
    fn exclude_all_local_matches_exclude_private_ips_list() {
        // Тот же набор, что «Exclude private IPs» в клиенте WireGuard для RFC 1918.
        let v = build_allowed_ips(exc(&NetPreset::WIDE), None).unwrap();
        for absent in [NET_10, NET_172, NET_192] {
            assert!(!v.contains(absent), "{v}");
        }
        assert!(v.starts_with("0.0.0.0/5, "), "{v}");
        assert!(v.contains("192.169.0.0/16"), "{v}");
        assert!(v.contains("172.32.0.0/11"), "{v}");
    }

    #[test]
    fn toggle_mode_flips_and_clears_all_and_vpn() {
        let mut sel = inc(&[NetPreset::Net192]);
        sel.vpn = true;
        sel.toggle_mode();
        assert_eq!(sel.mode, RouteMode::Exclude);
        assert!(!sel.vpn && sel.net(NetPreset::Net192));
        sel.toggle_mode();
        assert_eq!(sel.mode, RouteMode::Include);
        let mut all = RouteSelection {
            all: true,
            ..RouteSelection::default()
        };
        all.toggle_mode();
        assert!(all.is_empty());
    }

    #[test]
    fn toggle_all_or_vpn_in_exclude_mode_returns_to_include() {
        let mut sel = exc(&[NetPreset::Net10]);
        sel.toggle(RouteKey::All);
        assert_eq!(sel.mode, RouteMode::Include);
        assert!(sel.all);
        let mut sel = exc(&[NetPreset::Net10]);
        sel.toggle(RouteKey::Vpn);
        assert_eq!(sel.mode, RouteMode::Include);
        assert!(sel.vpn && sel.net(NetPreset::Net10));
    }

    #[test]
    fn selection_from_value_recognises_exclude_values() {
        for (nets, subnet) in [
            (vec![NetPreset::Net10], None),
            (vec![NetPreset::Net10], Some("10.9.9.0/24")),
            (NetPreset::WIDE.to_vec(), Some("10.9.9.0/24")),
            (vec![NetPreset::Net192_1, NetPreset::Net10_0], None),
            (vec![NetPreset::Net192_1], Some("192.168.1.128/25")),
        ] {
            let sel = exc(&nets);
            let v = build_allowed_ips(sel, subnet).unwrap();
            let back = selection_from_value(&v, subnet);
            assert_eq!(back, Some(sel), "{nets:?} / {subnet:?}: {v}");
        }
    }

    #[test]
    fn selection_from_value_exclude_drops_presets_nested_in_wider_ones() {
        // 192.168.1.0/24 внутри 192.168.0.0/16: строка та же, на экране —
        // только широкий тумблер.
        let v = build_allowed_ips(exc(&[NetPreset::Net192, NetPreset::Net192_1]), None).unwrap();
        assert_eq!(
            selection_from_value(&v, None),
            Some(exc(&[NetPreset::Net192]))
        );
    }

    #[test]
    fn selection_from_value_exclude_none_for_manual_lists() {
        // Дополнение чужой сети — не наш пресет.
        let manual = crate::vpn::cidr::subtract(
            &[crate::vpn::cidr::Ipv4Net::parse("0.0.0.0/0").unwrap()],
            &[crate::vpn::cidr::Ipv4Net::parse("1.2.3.0/24").unwrap()],
        )
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(", ");
        assert!(selection_from_value(&format!("{manual}, ::/0"), None).is_none());
        // Без ::/0 — тоже не наша сборка.
        let v = build_allowed_ips(exc(&[NetPreset::Net10]), None).unwrap();
        let no_v6 = v.trim_end_matches(", ::/0");
        assert!(selection_from_value(no_v6, None).is_none());
        // Мусор внутри списка.
        assert!(selection_from_value("0.0.0.0/5, garbage, ::/0", None).is_none());
    }

    #[test]
    fn network_cidr_masks_host_bits() {
        assert_eq!(network_cidr("10.9.9.1/24").unwrap(), "10.9.9.0/24");
        assert_eq!(network_cidr("10.8.0.1/29").unwrap(), "10.8.0.0/29");
        assert_eq!(network_cidr("192.168.5.77/16").unwrap(), "192.168.0.0/16");
        assert!(network_cidr("fd00::1/64").is_none());
        assert!(network_cidr("10.9.9.1").is_none());
        assert!(network_cidr("10.9.9.1/33").is_none());
    }

    #[test]
    fn vpn_subnet_from_addresses_takes_first_ipv4() {
        let addrs = vec!["fd00::1/64".to_string(), "10.9.9.1/24".to_string()];
        assert_eq!(vpn_subnet_from_addresses(&addrs).unwrap(), "10.9.9.0/24");
        assert!(vpn_subnet_from_addresses(&["fd00::1/64".to_string()]).is_none());
        assert!(vpn_subnet_from_addresses(&[]).is_none());
    }

    #[test]
    fn route_key_str_roundtrip() {
        for k in [
            RouteKey::All,
            RouteKey::Net(NetPreset::Net10),
            RouteKey::Net(NetPreset::Net192_100),
            RouteKey::Vpn,
            RouteKey::Local,
        ] {
            assert_eq!(RouteKey::parse_str(k.as_str()), Some(k));
        }
        assert!(RouteKey::parse_str("nope").is_none());
    }
}

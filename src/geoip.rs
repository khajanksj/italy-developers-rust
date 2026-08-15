//! IP-based country/locale detection. Cloudflare (the site's sole reverse
//! proxy) already computes `CF-IPCountry` for every request for free, so
//! that header is always tried first; a local MaxMind GeoLite2-Country
//! `.mmdb` file (path via `GEOIP_DB_PATH`) is the fallback for requests
//! that don't carry it (local dev, direct access). Every step degrades to
//! `None` instead of panicking — callers fall through to `Accept-Language`
//! and finally English (see `handlers::detect_lang`).

use std::net::IpAddr;

use actix_web::HttpRequest;

pub struct GeoState {
    reader: Option<maxminddb::Reader<Vec<u8>>>,
}

/// Loads the MaxMind database at `path`, if given. Never fails the caller —
/// a missing file, bad path, or corrupt database just disables the MaxMind
/// lookup layer (Cloudflare's header and Accept-Language still work).
pub fn load(path: Option<&str>) -> GeoState {
    let reader = path.and_then(|p| match maxminddb::Reader::open_readfile(p) {
        Ok(reader) => Some(reader),
        Err(err) => {
            tracing::warn!(error = %err, path = %p, "failed to load GeoIP database; MaxMind country lookup disabled");
            None
        }
    });
    GeoState { reader }
}

/// Best-effort real client IP: Cloudflare's `CF-Connecting-IP` (trustworthy
/// since Cloudflare is the confirmed sole reverse proxy in front of this
/// app and overwrites this header itself), else the TCP peer address.
pub fn client_ip(req: &HttpRequest) -> Option<IpAddr> {
    req.headers()
        .get("CF-Connecting-IP")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse().ok())
        .or_else(|| req.peer_addr().map(|addr| addr.ip()))
}

/// Two-letter ISO country code for this request, or `None` if it can't be
/// determined by any available signal.
pub fn country_for(req: &HttpRequest, geo: &GeoState) -> Option<String> {
    if let Some(header) = req
        .headers()
        .get("CF-IPCountry")
        .and_then(|v| v.to_str().ok())
    {
        let code = header.trim().to_uppercase();
        // Cloudflare's own "unknown" sentinels.
        if code.len() == 2 && code != "XX" && code != "T1" {
            return Some(code);
        }
    }
    let reader = geo.reader.as_ref()?;
    let ip = client_ip(req)?;
    let result = reader.lookup(ip).ok()?;
    let country: maxminddb::geoip2::Country = result.decode().ok().flatten()?;
    country
        .country
        .iso_code
        .map(|code| code.to_uppercase())
}

/// Maps a two-letter country code to one of the site's supported locales.
/// Unmapped countries return `None` so callers fall through to
/// `Accept-Language`/English rather than guessing.
pub fn locale_for_country(country: &str) -> Option<&'static str> {
    match country.to_uppercase().as_str() {
        "IT" => Some("it"),
        // Switzerland is multilingual; German is the largest share — an
        // approximation, not a claim of exclusivity.
        "DE" | "AT" | "CH" => Some("de"),
        "FR" | "MC" | "LU" => Some("fr"),
        "PT" | "BR" => Some("pt"),
        _ => None,
    }
}

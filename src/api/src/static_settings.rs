//! A read-only view of the configuration that is read at startup and cannot change while Rauthy
//! runs (config file, environment). It is built from an explicit list, so a new config value never
//! shows up here by accident, and secrets are only reported as `set` / `not set`.

use crate::ReqPrincipal;
use actix_web::{HttpResponse, get};
use rauthy_api_types::settings::{StaticSettingEntry, StaticSettingsResponse, StaticSettingsSection};
use rauthy_data::rauthy_config::RauthyConfig;
use rauthy_error::ErrorResponse;
use std::fmt::Debug;

struct Section {
    title: &'static str,
    entries: Vec<StaticSettingEntry>,
}

impl Section {
    fn new(title: &'static str) -> Self {
        Self {
            title,
            entries: Vec::new(),
        }
    }

    /// any value, shown the way it reads in a config file
    fn v<T: Debug>(mut self, key: &str, value: &T) -> Self {
        self.entries.push(StaticSettingEntry {
            key: key.to_string(),
            value: plain(value),
            secret: false,
        });
        self
    }

    /// a secret: only whether it is set
    fn s<T: Debug>(mut self, key: &str, value: &T) -> Self {
        let set = !matches!(plain(value).as_str(), "" | "-" | "[]");
        self.entries.push(StaticSettingEntry {
            key: key.to_string(),
            value: if set { "set" } else { "not set" }.to_string(),
            secret: true,
        });
        self
    }

    fn done(self) -> StaticSettingsSection {
        StaticSettingsSection {
            title: self.title.to_string(),
            entries: self.entries,
        }
    }
}

/// `Some("x")` -> `x`, `None` -> `-`, `"x"` -> `x`
fn plain<T: Debug>(value: &T) -> String {
    let s = format!("{value:?}");
    if s == "None" {
        return "-".to_string();
    }
    let s = s
        .strip_prefix("Some(")
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(&s);
    s.trim_matches('"').replace("\\\"", "\"")
}

/// GET the configuration that cannot be changed while Rauthy runs
///
/// Read only. Secrets are never returned, only whether they are set.
///
/// **Permissions**
/// - rauthy_admin
#[utoipa::path(
    get,
    path = "/settings/static",
    tag = "settings",
    responses(
        (status = 200, description = "Ok", body = StaticSettingsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
)]
#[get("/settings/static")]
pub async fn get_settings_static(principal: ReqPrincipal) -> Result<HttpResponse, ErrorResponse> {
    principal.validate_admin_session()?;

    let rc = RauthyConfig::get();
    let c = &rc.vars;

    let sections = vec![
        Section::new("Server")
            .v("server.pub_url", &c.server.pub_url)
            .v("server.scheme", &c.server.scheme)
            .v("server.listen_address", &c.server.listen_address)
            .v("server.port_http", &c.server.port_http)
            .v("server.port_https", &c.server.port_https)
            .v("server.http_workers", &c.server.http_workers)
            .v("server.proxy_mode", &c.server.proxy_mode)
            .v("server.trusted_proxies", &c.server.trusted_proxies)
            .v("server.metrics_enable", &c.server.metrics_enable)
            .v("server.metrics_addr", &c.server.metrics_addr)
            .v("server.metrics_port", &c.server.metrics_port)
            .v("server.scalar_enable", &c.server.scalar_enable)
            .v("server.scalar_public", &c.server.scalar_public)
            .v("server.book_public", &c.server.book_public)
            .v("server.ssp_threshold", &c.server.ssp_threshold)
            .v("issuer", &rc.issuer)
            .done(),
        Section::new("Database and cluster")
            .v("database.hiqlite", &c.database.hiqlite)
            .v("cluster.is_ha_cluster", &rc.is_ha_cluster)
            .v("cluster.is_primary_node", &rc.is_primary_node)
            .v("database.pg_host", &c.database.pg_host)
            .v("database.pg_port", &c.database.pg_port)
            .v("database.pg_user", &c.database.pg_user)
            .v("database.pg_db_name", &c.database.pg_db_name)
            .s("database.pg_password", &c.database.pg_password)
            .v("database.pg_max_conn", &c.database.pg_max_conn)
            .v("database.health_check_delay", &c.database.health_check_delay)
            .done(),
        Section::new("Encryption")
            .v("encryption.key_active", &c.encryption.key_active)
            .v(
                "encryption.keys (ids only)",
                &c.encryption
                    .keys
                    .iter()
                    // the part before the first `/` is the id, the rest is the key itself
                    .map(|k| k.split_once('/').map(|(id, _)| id).unwrap_or("?"))
                    .collect::<Vec<_>>()
                    .join(", "),
            )
            .done(),
        Section::new("Bootstrap")
            .v("bootstrap.admin_email", &c.bootstrap.admin_email)
            .s("bootstrap.password_plain", &c.bootstrap.password_plain)
            .s("bootstrap.password_argon2id", &c.bootstrap.password_argon2id)
            .s("bootstrap.api_key", &c.bootstrap.api_key)
            .done(),
        Section::new("WebAuthn")
            .v("webauthn.rp_id", &c.webauthn.rp_id)
            .v("webauthn.rp_origin", &c.webauthn.rp_origin)
            .v("webauthn.rp_name", &c.webauthn.rp_name)
            .v("webauthn.force_uv", &c.webauthn.force_uv)
            .v("webauthn.req_exp", &c.webauthn.req_exp)
            .v("webauthn.data_exp", &c.webauthn.data_exp)
            .done(),
        Section::new("Access and sessions")
            .v("access.cookie_mode", &c.access.cookie_mode)
            .v("access.cookie_set_path", &c.access.cookie_set_path)
            .v("access.session_validate_ip", &c.access.session_validate_ip)
            .v("access.peer_ip_header_name", &c.access.peer_ip_header_name)
            .v("access.userinfo_strict", &c.access.userinfo_strict)
            .v("access.token_len_limit", &c.access.token_len_limit)
            .v("access.token_revoke_on_logout", &c.access.token_revoke_on_logout)
            .v("access.admin_button_hide", &c.access.admin_button_hide)
            .v(
                "access.redirect_root_to_account",
                &c.access.redirect_root_to_account,
            )
            .v("mfa.admin_force_mfa", &c.mfa.admin_force_mfa)
            .done(),
        Section::new("Lifetimes")
            .v("lifetimes.session_lifetime", &c.lifetimes.session_lifetime)
            .v("lifetimes.session_timeout", &c.lifetimes.session_timeout)
            .v("lifetimes.refresh_token_lifetime", &c.lifetimes.refresh_token_lifetime)
            .v("lifetimes.refresh_token_grace_time", &c.lifetimes.refresh_token_grace_time)
            .v("lifetimes.magic_link_pwd_reset", &c.lifetimes.magic_link_pwd_reset)
            .v("lifetimes.magic_link_pwd_first", &c.lifetimes.magic_link_pwd_first)
            .v("lifetimes.jwk_autorotate_cron", &c.lifetimes.jwk_autorotate_cron)
            .done(),
        Section::new("Password hashing and proof of work")
            .v("hashing.argon2_m_cost", &c.hashing.argon2_m_cost)
            .v("hashing.argon2_t_cost", &c.hashing.argon2_t_cost)
            .v("hashing.argon2_p_cost", &c.hashing.argon2_p_cost)
            .v("hashing.max_hash_threads", &c.hashing.max_hash_threads)
            .v("pow.difficulty", &c.pow.difficulty)
            .v("pow.exp", &c.pow.exp)
            .done(),
        Section::new("Events")
            .v("events.email", &c.events.email)
            .v("events.notify_level_email", &c.events.notify_level_email)
            .v("events.notify_level_matrix", &c.events.notify_level_matrix)
            .v("events.notify_level_slack", &c.events.notify_level_slack)
            .v("events.persist_level", &c.events.persist_level)
            .v("events.cleanup_threshold", &c.events.cleanup_threshold)
            .s("events.slack_webhook", &c.events.slack_webhook)
            .s("events.matrix_access_token", &c.events.matrix_access_token)
            .v("events.disable_app_version_check", &c.events.disable_app_version_check)
            .done(),
        Section::new("Email (sending)")
            .v("email.rauthy_admin_email", &c.email.rauthy_admin_email)
            .v("email.sub_prefix", &c.email.sub_prefix)
            .v("email.connect_retries", &c.email.connect_retries)
            .v("email.smtp_conn_mode", &c.email.smtp_conn_mode)
            .v("email.password_exp", &c.email.password_exp)
            .done(),
        Section::new("User pictures")
            .v("user_pictures.storage_type", &c.user_pictures.storage_type)
            .v("user_pictures.path", &c.user_pictures.path)
            .v("user_pictures.s3_url", &c.user_pictures.s3_url)
            .v("user_pictures.bucket", &c.user_pictures.bucket)
            .v("user_pictures.region", &c.user_pictures.region)
            .s("user_pictures.s3_key", &c.user_pictures.s3_key)
            .s("user_pictures.s3_secret", &c.user_pictures.s3_secret)
            .v("user_pictures.upload_limit_mb", &c.user_pictures.upload_limit_mb)
            .v("user_pictures.public", &c.user_pictures.public)
            .done(),
        Section::new("Registration and deletion")
            .v("user_registration.enable", &c.user_registration.enable)
            .v("user_registration.domain_restriction", &c.user_registration.domain_restriction)
            .v("user_delete.enable_self_delete", &c.user_delete.enable_self_delete)
            .done(),
        Section::new("TLS")
            .v("tls.cert_path", &c.tls.cert_path)
            .v("tls.key_path", &c.tls.key_path)
            .v("tls.generate_self_signed", &c.tls.generate_self_signed)
            .done(),
        Section::new("Forward auth headers")
            .v("auth_headers.enable", &c.auth_headers.enable)
            .v("auth_headers.user", &c.auth_headers.user)
            .v("auth_headers.roles", &c.auth_headers.roles)
            .v("auth_headers.groups", &c.auth_headers.groups)
            .v("auth_headers.email", &c.auth_headers.email)
            .done(),
        Section::new("Logging")
            .v("logging.level", &c.logging.level)
            .v("logging.level_access", &c.logging.level_access)
            .v("logging.log_fmt", &c.logging.log_fmt)
            .done(),
        Section::new("Development flags")
            .v("dev.dev_mode", &c.dev.dev_mode)
            .v("dev.insecure_cookie", &c.dev.insecure_cookie)
            .v("dev.dpop_http", &c.dev.dpop_http)
            .done(),
    ];

    Ok(HttpResponse::Ok().json(StaticSettingsResponse {
        sections: sections
            .into_iter()
            .map(|s| StaticSettingsSection {
                title: s.title,
                entries: s.entries,
            })
            .collect(),
    }))
}

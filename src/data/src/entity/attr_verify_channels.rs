use crate::database::DB;
use crate::entity::user_attr::UserAttrConfigEntity;
use crate::settings;
use hiqlite::macros::{FromRow, params};
use rauthy_api_types::email_jobs::MessageChannel;
use rauthy_api_types::users::UserAttrConfigTyp;
use rauthy_common::is_hiqlite;
use rauthy_derive::FromPgRow;
use rauthy_error::{ErrorResponse, ErrorResponseType};
use serde::Deserialize;

/// Raw `attr_verify_channels` row, used by the DB migration tooling
#[derive(Debug, Deserialize, FromRow, FromPgRow)]
pub struct AttrVerifyChannelsRow {
    pub attr_name: String,
    pub channels: String,
}

/// The channels a verifiable attribute can be verified through. The first one is the default,
/// the user may pick another.
///
/// A verification code is delivered to the *value of the attribute* through the driver of the
/// channel. So a channel can only verify what it is able to deliver to, see [`Self::compatible`].
pub struct AttrVerifyChannels;

impl AttrVerifyChannels {
    pub async fn find_all() -> Result<Vec<AttrVerifyChannelsRow>, ErrorResponse> {
        let sql = "SELECT attr_name, channels FROM attr_verify_channels";
        Ok(if is_hiqlite() {
            DB::hql().query_map(sql, params!()).await?
        } else {
            DB::pg_query(sql, &[], 0).await?
        })
    }

    /// The configured links of an attribute, in order. Not filtered by availability.
    pub async fn find(attr: &str) -> Result<Vec<String>, ErrorResponse> {
        let sql = "SELECT attr_name, channels FROM attr_verify_channels WHERE attr_name = $1";
        let row: Option<AttrVerifyChannelsRow> = if is_hiqlite() {
            DB::hql().query_map_optional(sql, params!(attr)).await?
        } else {
            DB::pg_query_opt(sql, &[&attr]).await?
        };
        Ok(row.map(|r| split(&r.channels)).unwrap_or_default())
    }

    /// Whether `channel` is able to verify the value of an attribute of this type.
    ///
    /// - recipient `attr:<name>`: only the attribute `<name>`
    /// - recipient `email`: attributes of type `verified_email`
    /// - recipient `phone`: attributes of type `verified_phone`
    pub fn compatible(recipient: &str, attr: &str, typ: Option<UserAttrConfigTyp>) -> bool {
        match recipient {
            "email" => matches!(typ, Some(UserAttrConfigTyp::VerifiedEmail)),
            "phone" => matches!(typ, Some(UserAttrConfigTyp::VerifiedPhone)),
            other => other.strip_prefix("attr:") == Some(attr),
        }
    }

    /// The channels the user can choose from to verify the attribute: its links that exist,
    /// work, and can deliver to it. Without any links, `verified_email` falls back to `email`
    /// and `verified_phone` to `sms`, which is what they did before links existed.
    pub async fn usable(
        attr: &str,
        typ: Option<UserAttrConfigTyp>,
    ) -> Result<Vec<MessageChannel>, ErrorResponse> {
        let mut names = Self::find(attr).await?;
        if names.is_empty() {
            match typ {
                Some(UserAttrConfigTyp::VerifiedEmail) => names.push("email".into()),
                Some(UserAttrConfigTyp::VerifiedPhone) => names.push("sms".into()),
                _ => {}
            }
        }

        let mut out = Vec::with_capacity(names.len());
        for n in names {
            let Ok(ch) = n.parse::<MessageChannel>() else {
                continue;
            };
            let cfg = settings::channel(&ch);
            if !cfg.is_usable() {
                continue;
            }
            // The fallbacks above are the built-in channels whose recipient is the user's own
            // value, they deliver to any value of the matching kind.
            if cfg.recipient.starts_with("attr:") && cfg.recipient["attr:".len()..] != *attr {
                continue;
            }
            if !cfg.recipient.starts_with("attr:") {
                let ok = match typ {
                    Some(UserAttrConfigTyp::VerifiedEmail) => cfg.recipient == "email",
                    Some(UserAttrConfigTyp::VerifiedPhone) => cfg.recipient == "phone",
                    _ => false,
                };
                if !ok {
                    continue;
                }
            }
            out.push(ch);
        }
        Ok(out)
    }

    /// All channels that exist and are able to deliver to the attribute, whether or not they
    /// are linked yet.
    pub async fn candidates(attr: &str, typ: Option<UserAttrConfigTyp>) -> Vec<MessageChannel> {
        settings::all_channels()
            .into_iter()
            .filter(|ch| Self::compatible(&settings::channel(ch).recipient, attr, typ.clone()))
            .collect()
    }

    /// Replaces the links of an attribute. An empty list removes them.
    pub async fn save(attr: &str, channels: Vec<String>) -> Result<Vec<String>, ErrorResponse> {
        let cfg = UserAttrConfigEntity::find(attr.to_string()).await?;
        if !matches!(
            cfg.typ,
            Some(
                UserAttrConfigTyp::VerifiedEmail
                    | UserAttrConfigTyp::VerifiedPhone
                    | UserAttrConfigTyp::Verified
            )
        ) {
            return Err(bad("Only verifiable attributes can be linked to channels"));
        }

        let mut clean: Vec<String> = Vec::with_capacity(channels.len());
        for c in channels {
            let c = c.trim().to_string();
            let ch = c
                .parse::<MessageChannel>()
                .map_err(|_| bad(format!("invalid channel '{c}'")))?;
            if !settings::all_channels().contains(&ch) {
                return Err(bad(format!("unknown channel '{c}'")));
            }
            let recipient = settings::channel(&ch).recipient;
            if !Self::compatible(&recipient, attr, cfg.typ.clone()) {
                return Err(bad(format!(
                    "The channel '{c}' delivers to '{recipient}' and cannot verify '{attr}'"
                )));
            }
            if !clean.contains(&c) {
                clean.push(c);
            }
        }

        if clean.is_empty() {
            let sql = "DELETE FROM attr_verify_channels WHERE attr_name = $1";
            if is_hiqlite() {
                DB::hql().execute(sql, params!(attr)).await?;
            } else {
                DB::pg_execute(sql, &[&attr]).await?;
            }
        } else {
            let joined = clean.join(",");
            let sql = r#"
INSERT INTO attr_verify_channels (attr_name, channels)
VALUES ($1, $2)
ON CONFLICT (attr_name) DO UPDATE SET channels = $2"#;
            if is_hiqlite() {
                DB::hql().execute(sql, params!(attr, joined)).await?;
            } else {
                DB::pg_execute(sql, &[&attr, &joined]).await?;
            }
        }
        Ok(clean)
    }

    /// Drops a deleted channel from every attribute
    pub async fn remove_channel(channel: &str) -> Result<(), ErrorResponse> {
        for row in Self::find_all().await? {
            let list = split(&row.channels);
            if list.iter().any(|c| c == channel) {
                let rest: Vec<String> = list.into_iter().filter(|c| c != channel).collect();
                // validation is skipped on purpose, only links are removed here
                let sql = if rest.is_empty() {
                    "DELETE FROM attr_verify_channels WHERE attr_name = $1"
                } else {
                    "UPDATE attr_verify_channels SET channels = $2 WHERE attr_name = $1"
                };
                let joined = rest.join(",");
                if is_hiqlite() {
                    if rest.is_empty() {
                        DB::hql().execute(sql, params!(row.attr_name)).await?;
                    } else {
                        DB::hql()
                            .execute(sql, params!(row.attr_name, joined))
                            .await?;
                    }
                } else if rest.is_empty() {
                    DB::pg_execute(sql, &[&row.attr_name]).await?;
                } else {
                    DB::pg_execute(sql, &[&row.attr_name, &joined]).await?;
                }
            }
        }
        Ok(())
    }
}

fn split(s: &str) -> Vec<String> {
    s.split(',')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

fn bad(msg: impl Into<String>) -> ErrorResponse {
    ErrorResponse::new(ErrorResponseType::BadRequest, msg.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_compatibility() {
        use UserAttrConfigTyp::*;
        assert!(AttrVerifyChannels::compatible(
            "email",
            "x",
            Some(VerifiedEmail)
        ));
        assert!(!AttrVerifyChannels::compatible(
            "email",
            "x",
            Some(VerifiedPhone)
        ));
        assert!(AttrVerifyChannels::compatible(
            "phone",
            "x",
            Some(VerifiedPhone)
        ));
        assert!(AttrVerifyChannels::compatible(
            "attr:wa",
            "wa",
            Some(Verified)
        ));
        assert!(!AttrVerifyChannels::compatible(
            "attr:wa",
            "other",
            Some(Verified)
        ));
        assert!(!AttrVerifyChannels::compatible(
            "phone",
            "wa",
            Some(Verified)
        ));
    }
}

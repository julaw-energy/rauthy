//! Per-client rules on user values and attributes that decide who may log in.
//!
//! A client can say "only users from UG, KE and TZ" or "only users of 18 and older". The rules
//! of a client are a short list, all of them must hold. They are checked once per login in
//! `finish_authorize`, right next to the group check. The check is cheap:
//!
//! - a client without rules costs one in-memory cache lookup,
//! - the user values and attributes are only loaded when a rule needs them (both are cached),
//! - evaluating a rule is a few string / number comparisons, no allocation worth mentioning.
//!
//! The comparison itself is a pure function ([`holds`]), so it is tested without a database.
//!
//! A value that is missing never satisfies a rule, except for `not_set`. That makes the rules
//! deny by default: "country in UG" does not let in a user who has no country.

use crate::database::{Cache, DB};
use crate::entity::groups::Group;
use crate::entity::roles::Role;
use crate::entity::user_attr::{UserAttrConfigEntity, UserAttrValueEntity};
use crate::entity::users::User;
use crate::entity::users_values::UserValues;
use chrono::{Datelike, NaiveDate, Utc};
use hiqlite::macros::{FromRow, params};
use rauthy_api_types::clients::{AccessRuleField, ClientAccessRule};
use rauthy_api_types::users::UserAttrConfigTyp;
use rauthy_common::constants::CACHE_TTL_APP;
use rauthy_common::is_hiqlite;
use rauthy_derive::FromPgRow;
use rauthy_error::{ErrorResponse, ErrorResponseType};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use tracing::warn;

pub const MAX_RULES: usize = 20;
const MAX_LIST: usize = 100;
const MAX_TEXT: usize = 256;
const MAX_MESSAGE: usize = 200;

/// What a user hears when a rule without its own message refuses the login
pub const DEFAULT_MESSAGE: &str =
    "Your account does not meet the requirements to access this application";

pub const OPS: &[&str] = &[
    "eq",
    "ne",
    "gt",
    "gte",
    "lt",
    "lte",
    "between",
    "in",
    "not_in",
    "contains",
    "starts_with",
    "ends_with",
    "is_set",
    "not_set",
];

/// The built-in fields. Custom attributes are `attr:<name>`.
pub const FIELDS: &[&str] = &[
    "given_name",
    "family_name",
    "preferred_username",
    "email",
    "email_verified",
    "birthdate",
    "age",
    "gender",
    "tz",
    "phone",
    "street",
    "zip",
    "city",
    "country",
    "roles",
    "groups",
];

/// Raw `client_access_rules` row, used by the DB migration tooling
#[derive(Debug, Deserialize, FromRow, FromPgRow)]
pub struct ClientAccessRulesRow {
    pub client_id: String,
    pub rules: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Between,
    In,
    NotIn,
    Contains,
    StartsWith,
    EndsWith,
    IsSet,
    NotSet,
}

impl Op {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "eq" => Self::Eq,
            "ne" => Self::Ne,
            "gt" => Self::Gt,
            "gte" => Self::Gte,
            "lt" => Self::Lt,
            "lte" => Self::Lte,
            "between" => Self::Between,
            "in" => Self::In,
            "not_in" => Self::NotIn,
            "contains" => Self::Contains,
            "starts_with" => Self::StartsWith,
            "ends_with" => Self::EndsWith,
            "is_set" => Self::IsSet,
            "not_set" => Self::NotSet,
            _ => return None,
        })
    }

    fn takes_list(self) -> bool {
        matches!(self, Self::In | Self::NotIn)
    }

    fn takes_nothing(self) -> bool {
        matches!(self, Self::IsSet | Self::NotSet)
    }

    fn is_numeric(self) -> bool {
        matches!(self, Self::Gt | Self::Gte | Self::Lt | Self::Lte)
    }
}

/// A value of a rule: its text and, if it is a number, the number
#[derive(Debug, Clone, PartialEq)]
struct Scalar {
    text: String,
    num: Option<f64>,
}

impl Scalar {
    fn from_text(s: &str) -> Self {
        Self {
            text: s.trim().to_string(),
            num: s.trim().parse::<f64>().ok().filter(|n| n.is_finite()),
        }
    }

    fn from_json(v: &Value) -> Option<Self> {
        match v {
            Value::String(s) if s.len() <= MAX_TEXT => Some(Self::from_text(s)),
            Value::Number(n) => {
                let num = n.as_f64().filter(|n| n.is_finite())?;
                Some(Self {
                    text: n.to_string(),
                    num: Some(num),
                })
            }
            Value::Bool(b) => Some(Self {
                text: b.to_string(),
                num: None,
            }),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
enum Expected {
    Nothing,
    One(Scalar),
    Many(Vec<Scalar>),
}

/// What the user has for a field
#[derive(Debug, Clone, PartialEq)]
pub enum Actual {
    Missing,
    Text(String),
    List(Vec<String>),
    Bool(bool),
}

impl Actual {
    fn is_set(&self) -> bool {
        match self {
            Self::Missing => false,
            Self::Text(t) => !t.trim().is_empty(),
            Self::List(l) => !l.is_empty(),
            Self::Bool(_) => true,
        }
    }

    /// `f` holds for the value, or for any element of a list
    fn any(&self, f: impl Fn(&str) -> bool) -> bool {
        match self {
            Self::Missing => false,
            Self::Text(t) => f(t),
            Self::List(l) => l.iter().any(|e| f(e)),
            Self::Bool(b) => f(if *b { "true" } else { "false" }),
        }
    }
}

/// Numbers compare as numbers (`"18"` equals `18.0`), anything else as text without case
fn scalar_eq(actual: &str, expected: &Scalar) -> bool {
    let actual = actual.trim();
    match (
        actual.parse::<f64>().ok().filter(|n| n.is_finite()),
        expected.num,
    ) {
        (Some(a), Some(e)) => a == e,
        _ => actual.eq_ignore_ascii_case(&expected.text),
    }
}

/// Whether one rule holds for what the user has. A missing value fails every rule but `not_set`.
fn holds(op: Op, actual: &Actual, expected: &Expected) -> bool {
    match op {
        Op::IsSet => return actual.is_set(),
        Op::NotSet => return !actual.is_set(),
        _ => {}
    }
    if !actual.is_set() {
        return false;
    }

    match (op, expected) {
        (Op::Eq, Expected::One(e)) => actual.any(|a| scalar_eq(a, e)),
        (Op::Ne, Expected::One(e)) => !actual.any(|a| scalar_eq(a, e)),
        (Op::In, Expected::Many(list)) => actual.any(|a| list.iter().any(|e| scalar_eq(a, e))),
        (Op::NotIn, Expected::Many(list)) => !actual.any(|a| list.iter().any(|e| scalar_eq(a, e))),
        (Op::Gt | Op::Gte | Op::Lt | Op::Lte, Expected::One(e)) => {
            let (Actual::Text(a), Some(e)) = (actual, e.num) else {
                return false;
            };
            let Some(a) = a.trim().parse::<f64>().ok().filter(|n| n.is_finite()) else {
                return false;
            };
            match op {
                Op::Gt => a > e,
                Op::Gte => a >= e,
                Op::Lt => a < e,
                _ => a <= e,
            }
        }
        (Op::Between, Expected::Many(range)) => {
            let (Actual::Text(a), [lo, hi]) = (actual, range.as_slice()) else {
                return false;
            };
            let (Some(lo), Some(hi)) = (lo.num, hi.num) else {
                return false;
            };
            let Some(a) = a.trim().parse::<f64>().ok().filter(|n| n.is_finite()) else {
                return false;
            };
            lo <= a && a <= hi
        }
        (Op::Contains, Expected::One(e)) => match actual {
            Actual::Text(t) => t.to_lowercase().contains(&e.text.to_lowercase()),
            Actual::List(l) => l.iter().any(|x| scalar_eq(x, e)),
            _ => false,
        },
        (Op::StartsWith, Expected::One(e)) => match actual {
            Actual::Text(t) => t.trim().to_lowercase().starts_with(&e.text.to_lowercase()),
            _ => false,
        },
        (Op::EndsWith, Expected::One(e)) => match actual {
            Actual::Text(t) => t.trim().to_lowercase().ends_with(&e.text.to_lowercase()),
            _ => false,
        },
        _ => false,
    }
}

/// Checks the shape of a rule and turns it into what [`holds`] needs
fn compile(rule: &ClientAccessRule) -> Result<(Op, Expected), String> {
    let op = Op::parse(&rule.op).ok_or_else(|| format!("unknown operator '{}'", rule.op))?;
    let value = rule.value.as_ref().filter(|v| !v.is_null());

    let expected = if op.takes_nothing() {
        if value.is_some() {
            return Err(format!("'{}' takes no value", rule.op));
        }
        Expected::Nothing
    } else if op == Op::Between {
        let Some(Value::Array(items)) = value else {
            return Err("'between' needs a list with the lowest and the highest value".to_string());
        };
        let [lo, hi] = items.as_slice() else {
            return Err(
                "'between' needs exactly two values, the lowest and the highest".to_string(),
            );
        };
        let (Some(lo), Some(hi)) = (Scalar::from_json(lo), Scalar::from_json(hi)) else {
            return Err("'between' compares numbers".to_string());
        };
        let (Some(a), Some(b)) = (lo.num, hi.num) else {
            return Err("'between' compares numbers, the values are not numbers".to_string());
        };
        if a > b {
            return Err("'between' needs the lowest value first".to_string());
        }
        Expected::Many(vec![lo, hi])
    } else if op.takes_list() {
        let Some(Value::Array(items)) = value else {
            return Err(format!("'{}' needs a list of values", rule.op));
        };
        if items.is_empty() || items.len() > MAX_LIST {
            return Err(format!("'{}' needs 1 to {MAX_LIST} values", rule.op));
        }
        let list = items
            .iter()
            .map(Scalar::from_json)
            .collect::<Option<Vec<_>>>()
            .ok_or("a list can only hold texts and numbers")?;
        Expected::Many(list)
    } else {
        let Some(v) = value else {
            return Err(format!("'{}' needs a value", rule.op));
        };
        let scalar =
            Scalar::from_json(v).ok_or("the value must be a text, a number or a boolean")?;
        if op.is_numeric() && scalar.num.is_none() {
            return Err(format!(
                "'{}' compares numbers, the value is not one",
                rule.op
            ));
        }
        Expected::One(scalar)
    };

    Ok((op, expected))
}

/// Evaluates the rules with `get` as the source of the user's values. Returns the first rule that
/// does not hold, as `field op`.
pub fn first_failing<'a>(
    rules: &'a [ClientAccessRule],
    get: &dyn Fn(&str) -> Actual,
) -> Option<&'a ClientAccessRule> {
    for rule in rules {
        let Ok((op, expected)) = compile(rule) else {
            // a stored rule that does not compile cannot be met, so nobody gets in by mistake
            return Some(rule);
        };
        if !holds(op, &get(&rule.field), &expected) {
            return Some(rule);
        }
    }
    None
}

/// Full years since `birthdate` (`YYYY-MM-DD`), `None` for anything that is not a past date
fn age_on(birthdate: &str, today: NaiveDate) -> Option<i32> {
    let b = NaiveDate::parse_from_str(birthdate.trim(), "%Y-%m-%d").ok()?;
    if b > today {
        return None;
    }
    let mut age = today.year() - b.year();
    if (today.month(), today.day()) < (b.month(), b.day()) {
        age -= 1;
    }
    Some(age)
}

fn actual_from_json(bytes: &[u8]) -> Actual {
    match serde_json::from_slice::<Value>(bytes) {
        Ok(Value::String(s)) => Actual::Text(s),
        Ok(Value::Number(n)) => Actual::Text(n.to_string()),
        Ok(Value::Bool(b)) => Actual::Bool(b),
        Ok(Value::Array(a)) => Actual::List(
            a.iter()
                .filter_map(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(n) => Some(n.to_string()),
                    Value::Bool(b) => Some(b.to_string()),
                    _ => None,
                })
                .collect(),
        ),
        _ => Actual::Missing,
    }
}

fn opt_text(v: &Option<String>) -> Actual {
    v.as_ref()
        .map(|s| Actual::Text(s.clone()))
        .unwrap_or(Actual::Missing)
}

fn csv_list(v: &str) -> Actual {
    Actual::List(
        v.split(',')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect(),
    )
}

pub struct ClientAccessRules;

impl ClientAccessRules {
    fn cache_idx(client_id: &str) -> String {
        format!("client_access_rules_{client_id}")
    }

    fn parse(json: &str) -> Vec<ClientAccessRule> {
        serde_json::from_str(json).unwrap_or_default()
    }

    /// The rules of a client. Cached as the JSON text, an empty list is cached too, so a client
    /// without rules does not cost a query per login.
    pub async fn find(client_id: &str) -> Result<Vec<ClientAccessRule>, ErrorResponse> {
        let idx = Self::cache_idx(client_id);
        let cached: Option<String> = DB::hql().get(Cache::App, idx.clone()).await?;
        if let Some(json) = cached {
            return Ok(Self::parse(&json));
        }

        let sql = "SELECT client_id, rules FROM client_access_rules WHERE client_id = $1";
        let row: Option<ClientAccessRulesRow> = if is_hiqlite() {
            DB::hql()
                .query_map_optional(sql, params!(client_id))
                .await?
        } else {
            DB::pg_query_opt(sql, &[&client_id]).await?
        };
        let json = row.map(|r| r.rules).unwrap_or_else(|| "[]".to_string());

        DB::hql().put(Cache::App, idx, &json, CACHE_TTL_APP).await?;
        Ok(Self::parse(&json))
    }

    /// The fields a rule can use, with their type and choices, so the UI can offer the right
    /// input: the built-in fields and the custom attributes that exist.
    pub async fn available_fields() -> Result<Vec<AccessRuleField>, ErrorResponse> {
        let field = |name: &str, kind: &str, options: Vec<String>| AccessRuleField {
            name: name.to_string(),
            kind: kind.to_string(),
            options,
        };

        let mut roles: Vec<String> = Role::find_all()
            .await?
            .into_iter()
            .map(|r| r.name)
            .collect();
        roles.sort();
        let mut groups: Vec<String> = Group::find_all()
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect();
        groups.sort();

        let mut fields: Vec<AccessRuleField> = FIELDS
            .iter()
            .map(|name| match *name {
                "email_verified" => field(name, "bool", vec![]),
                "birthdate" => field(name, "date", vec![]),
                "age" => field(name, "number", vec![]),
                "country" => field(name, "country", vec![]),
                "gender" => field(
                    name,
                    "choice",
                    ["female", "male", "other"].map(String::from).to_vec(),
                ),
                "roles" => field(name, "list", roles.clone()),
                "groups" => field(name, "list", groups.clone()),
                _ => field(name, "text", vec![]),
            })
            .collect();

        let mut attrs = UserAttrConfigEntity::find_all().await?;
        attrs.sort_by(|a, b| a.name.cmp(&b.name));
        for a in attrs {
            let name = format!("attr:{}", a.name);
            fields.push(match a.typ {
                Some(UserAttrConfigTyp::Boolean) => field(&name, "bool", vec![]),
                Some(UserAttrConfigTyp::Number | UserAttrConfigTyp::Integer) => {
                    field(&name, "number", vec![])
                }
                Some(UserAttrConfigTyp::Date) => field(&name, "date", vec![]),
                Some(UserAttrConfigTyp::Country) => field(&name, "country", vec![]),
                Some(UserAttrConfigTyp::Enum) => {
                    let options = a
                        .options
                        .as_deref()
                        .and_then(|o| serde_json::from_str::<Vec<String>>(o).ok())
                        .unwrap_or_default();
                    field(&name, "choice", options)
                }
                _ => field(&name, "text", vec![]),
            });
        }
        Ok(fields)
    }

    /// Replaces the rules of a client, an empty list removes them.
    pub async fn save(
        client_id: &str,
        rules: Vec<ClientAccessRule>,
    ) -> Result<Vec<ClientAccessRule>, ErrorResponse> {
        let attrs = UserAttrConfigEntity::find_all_as_set().await?;
        validate(&rules, &attrs)
            .map_err(|msg| ErrorResponse::new(ErrorResponseType::BadRequest, msg))?;

        if rules.is_empty() {
            let sql = "DELETE FROM client_access_rules WHERE client_id = $1";
            if is_hiqlite() {
                DB::hql().execute(sql, params!(client_id)).await?;
            } else {
                DB::pg_execute(sql, &[&client_id]).await?;
            }
        } else {
            let json = serde_json::to_string(&rules)?;
            let sql = r#"
INSERT INTO client_access_rules (client_id, rules)
VALUES ($1, $2)
ON CONFLICT (client_id) DO UPDATE SET rules = $2"#;
            if is_hiqlite() {
                DB::hql().execute(sql, params!(client_id, json)).await?;
            } else {
                DB::pg_execute(sql, &[&client_id, &json]).await?;
            }
        }

        DB::hql()
            .delete(Cache::App, Self::cache_idx(client_id))
            .await?;
        Ok(rules)
    }

    /// Lets a login through only if the user meets every rule of the client.
    ///
    /// The `rauthy` client is never checked: the admin UI must stay reachable.
    pub async fn validate_user(user: &User, client_id: &str) -> Result<(), ErrorResponse> {
        if client_id == "rauthy" {
            return Ok(());
        }
        let rules = Self::find(client_id).await?;
        if rules.is_empty() {
            return Ok(());
        }

        // load what the rules need, and nothing else
        let uses = |f: &dyn Fn(&str) -> bool| rules.iter().any(|r| f(&r.field));
        let needs_values = uses(&|f| {
            matches!(
                f,
                "preferred_username"
                    | "birthdate"
                    | "age"
                    | "gender"
                    | "tz"
                    | "phone"
                    | "street"
                    | "zip"
                    | "city"
                    | "country"
            )
        });
        let uv = if needs_values {
            UserValues::find(&user.id).await?.unwrap_or_default()
        } else {
            UserValues::default()
        };
        let attrs = if uses(&|f| f.starts_with("attr:")) {
            UserAttrValueEntity::find_for_user(&user.id).await?
        } else {
            Vec::new()
        };
        let today = Utc::now().date_naive();

        let get = |field: &str| -> Actual {
            match field {
                "given_name" => Actual::Text(user.given_name.clone()),
                "family_name" => opt_text(&user.family_name),
                "email" => Actual::Text(user.email.clone()),
                "email_verified" => Actual::Bool(user.email_verified),
                "roles" => csv_list(&user.roles),
                "groups" => csv_list(user.groups.as_deref().unwrap_or_default()),
                "preferred_username" => opt_text(&uv.preferred_username),
                "birthdate" => opt_text(&uv.birthdate),
                "age" => uv
                    .birthdate
                    .as_deref()
                    .and_then(|b| age_on(b, today))
                    .map(|a| Actual::Text(a.to_string()))
                    .unwrap_or(Actual::Missing),
                "gender" => opt_text(&uv.gender),
                "tz" => opt_text(&uv.tz),
                "phone" => opt_text(&uv.phone),
                "street" => opt_text(&uv.street),
                "zip" => opt_text(&uv.zip),
                "city" => opt_text(&uv.city),
                "country" => opt_text(&uv.country),
                other => other
                    .strip_prefix("attr:")
                    .and_then(|name| attrs.iter().find(|a| a.key == name))
                    .map(|a| actual_from_json(&a.value))
                    .unwrap_or(Actual::Missing),
            }
        };

        match first_failing(&rules, &get) {
            None => Ok(()),
            Some(rule) => {
                // the log says which rule, the user only hears that the account does not qualify
                warn!(
                    user_id = user.id,
                    client_id,
                    field = rule.field,
                    op = rule.op,
                    "Login refused: the user does not meet an access rule of the client"
                );
                Err(ErrorResponse::new(
                    ErrorResponseType::Forbidden,
                    message_for(rule),
                ))
            }
        }
    }
}

/// The text for the user when `rule` refuses the login: the one the admin wrote, or the general one
fn message_for(rule: &ClientAccessRule) -> String {
    rule.message
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .unwrap_or(DEFAULT_MESSAGE)
        .to_string()
}

/// Checks a list of rules before it is saved
pub fn validate(rules: &[ClientAccessRule], attrs: &HashSet<String>) -> Result<(), String> {
    if rules.len() > MAX_RULES {
        return Err(format!("A client can have at most {MAX_RULES} rules"));
    }
    for r in rules {
        let known = match r.field.strip_prefix("attr:") {
            Some(name) => attrs.contains(name),
            None => FIELDS.contains(&r.field.as_str()),
        };
        if !known {
            return Err(format!("unknown field '{}'", r.field));
        }
        compile(r).map_err(|e| format!("{}: {e}", r.field))?;
        if let Some(m) = &r.message
            && (m.chars().count() > MAX_MESSAGE || m.chars().any(char::is_control))
        {
            return Err(format!(
                "{}: the message must be a single line of at most {MAX_MESSAGE} characters",
                r.field
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rule(field: &str, op: &str, value: Option<Value>) -> ClientAccessRule {
        ClientAccessRule {
            field: field.into(),
            op: op.into(),
            value,
            message: None,
        }
    }

    fn run(op: &str, actual: Actual, value: Option<Value>) -> bool {
        let (op, expected) = compile(&rule("x", op, value)).expect("rule compiles");
        holds(op, &actual, &expected)
    }

    fn text(s: &str) -> Actual {
        Actual::Text(s.into())
    }

    #[test]
    fn countries_in_a_list() {
        let list = Some(json!(["UG", "KE", "TZ"]));
        assert!(run("in", text("UG"), list.clone()));
        assert!(run("in", text("ke"), list.clone()), "case does not matter");
        assert!(
            run("in", text(" TZ "), list.clone()),
            "spaces do not matter"
        );
        assert!(!run("in", text("RW"), list.clone()));
        assert!(
            !run("in", Actual::Missing, list.clone()),
            "no country, no access"
        );
        assert!(
            !run("in", text(""), list.clone()),
            "an empty country is no country"
        );
        assert!(run("not_in", text("RW"), list.clone()));
        assert!(!run("not_in", text("UG"), list.clone()));
        assert!(
            !run("not_in", Actual::Missing, list),
            "missing never passes"
        );
    }

    #[test]
    fn age_and_numbers() {
        assert!(run("gte", text("18"), Some(json!(18))));
        assert!(run("gte", text("40"), Some(json!(18))));
        assert!(!run("gte", text("17"), Some(json!(18))));
        assert!(run("gt", text("19"), Some(json!(18))));
        assert!(!run("gt", text("18"), Some(json!(18))));
        assert!(run("lt", text("17"), Some(json!(18))));
        assert!(run("lte", text("18"), Some(json!(18))));
        // a number given as text works, so does a decimal
        assert!(run("gte", text("18.5"), Some(json!("18"))));
        // not a number: fails, does not panic
        assert!(!run("gte", text("abc"), Some(json!(18))));
        assert!(!run("gte", text("NaN"), Some(json!(18))));
        assert!(!run("gte", text("inf"), Some(json!(18))));
        assert!(!run("gte", Actual::Missing, Some(json!(18))));
        assert!(!run(
            "gte",
            Actual::List(vec!["20".into()]),
            Some(json!(18))
        ));
    }

    #[test]
    fn equality_of_text_numbers_and_bools() {
        assert!(run("eq", text("Female"), Some(json!("female"))));
        assert!(run("ne", text("Female"), Some(json!("male"))));
        assert!(!run("ne", Actual::Missing, Some(json!("male"))));
        assert!(run("eq", text("18"), Some(json!(18))), "18 equals 18");
        assert!(run("eq", text("18.0"), Some(json!(18))), "18.0 equals 18");
        assert!(run("eq", Actual::Bool(true), Some(json!(true))));
        assert!(!run("eq", Actual::Bool(false), Some(json!(true))));
        assert!(run("eq", text("true"), Some(json!(true))));
    }

    #[test]
    fn lists_of_roles_and_groups() {
        let roles = Actual::List(vec!["user".into(), "staff".into()]);
        assert!(run("contains", roles.clone(), Some(json!("staff"))));
        assert!(!run("contains", roles.clone(), Some(json!("admin"))));
        assert!(run("in", roles.clone(), Some(json!(["admin", "staff"]))));
        assert!(!run("in", roles.clone(), Some(json!(["admin"]))));
        assert!(run("not_in", roles.clone(), Some(json!(["admin"]))));
        assert!(!run("not_in", roles.clone(), Some(json!(["staff"]))));
        assert!(run("eq", roles, Some(json!("user"))), "any element");
        assert!(
            !run("is_set", Actual::List(vec![]), None),
            "an empty list is not set"
        );
    }

    fn with_message(mut r: ClientAccessRule, m: Option<&str>) -> ClientAccessRule {
        r.message = m.map(String::from);
        r
    }

    #[test]
    fn the_failing_rule_decides_the_message() {
        let attrs = HashSet::new();
        let a = with_message(
            rule("country", "in", Some(json!(["UG"]))),
            Some("  Only for Uganda  "),
        );
        let b = with_message(rule("age", "gte", Some(json!(18))), Some("   "));
        let c = rule("email", "is_set", None);
        assert!(validate(&[a.clone(), b.clone(), c.clone()], &attrs).is_ok());
        assert_eq!(message_for(&a), "Only for Uganda");
        // blank or missing: the general message
        assert_eq!(message_for(&b), DEFAULT_MESSAGE);
        assert_eq!(message_for(&c), DEFAULT_MESSAGE);

        // rules are tried in order: the first one that fails speaks
        let get = |f: &str| match f {
            "country" => Actual::Text("DE".into()),
            "age" => Actual::Text("12".into()),
            _ => Actual::Missing,
        };
        let rules = [b.clone(), a.clone()];
        assert_eq!(
            message_for(first_failing(&rules, &get).unwrap()),
            DEFAULT_MESSAGE
        );
        let rules = [a, b];
        assert_eq!(
            message_for(first_failing(&rules, &get).unwrap()),
            "Only for Uganda"
        );
    }

    #[test]
    fn a_message_must_be_one_short_line() {
        let attrs = HashSet::new();
        let ok = |m: String| {
            validate(
                &[with_message(rule("email", "is_set", None), Some(&m))],
                &attrs,
            )
            .is_ok()
        };
        assert!(ok("x".repeat(MAX_MESSAGE)));
        assert!(ok("Nur für Nutzer aus Österreich".into()));
        assert!(!ok("x".repeat(MAX_MESSAGE + 1)));
        assert!(!ok("two\nlines".into()));
    }

    #[test]
    fn a_range_includes_both_ends() {
        let r = Some(json!([18, 65]));
        assert!(run("between", text("18"), r.clone()), "the lower end is in");
        assert!(run("between", text("65"), r.clone()), "the upper end is in");
        assert!(run("between", text("40"), r.clone()));
        assert!(run("between", text("17.99"), Some(json!([17.5, 18]))));
        assert!(!run("between", text("17"), r.clone()));
        assert!(!run("between", text("66"), r.clone()));
        assert!(!run("between", text("abc"), r.clone()));
        assert!(!run("between", text("NaN"), r.clone()));
        assert!(
            !run("between", Actual::Missing, r.clone()),
            "missing never passes"
        );
        assert!(!run("between", Actual::List(vec!["30".into()]), r));
        // numbers given as text are fine
        assert!(run("between", text("30"), Some(json!(["18", "65"]))));
        // a range of one value
        assert!(run("between", text("18"), Some(json!([18, 18]))));
    }

    #[test]
    fn text_operators() {
        assert!(run(
            "contains",
            text("Jane@Example.com"),
            Some(json!("example"))
        ));
        assert!(run(
            "starts_with",
            text("Jane@example.com"),
            Some(json!("jane"))
        ));
        assert!(run(
            "ends_with",
            text("jane@example.com"),
            Some(json!("@EXAMPLE.com"))
        ));
        assert!(!run(
            "ends_with",
            text("jane@other.com"),
            Some(json!("@example.com"))
        ));
        assert!(!run(
            "starts_with",
            Actual::List(vec!["a".into()]),
            Some(json!("a"))
        ));
    }

    #[test]
    fn set_and_not_set() {
        assert!(run("is_set", text("x"), None));
        assert!(!run("is_set", text("   "), None));
        assert!(!run("is_set", Actual::Missing, None));
        assert!(run("is_set", Actual::Bool(false), None), "false is a value");
        assert!(run("not_set", Actual::Missing, None));
        assert!(run("not_set", text(""), None));
        assert!(!run("not_set", text("x"), None));
    }

    #[test]
    fn age_from_the_birthdate() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        assert_eq!(age_on("2008-10-27", today), Some(18), "birthday today");
        assert_eq!(age_on("2008-10-28", today), Some(17), "birthday tomorrow");
        assert_eq!(age_on("1990-01-01", today), Some(36));
        assert_eq!(age_on("2000-02-29", today), Some(26));
        assert_eq!(age_on("2030-01-01", today), None, "in the future");
        assert_eq!(age_on("not a date", today), None);
        assert_eq!(age_on("", today), None);
    }

    #[test]
    fn all_rules_must_hold_and_the_first_failure_is_returned() {
        let rules = vec![
            rule("country", "in", Some(json!(["UG", "KE"]))),
            rule("age", "gte", Some(json!(18))),
        ];
        let user = |country: &str, age: &str| {
            let (country, age) = (country.to_string(), age.to_string());
            move |f: &str| match f {
                "country" => Actual::Text(country.clone()),
                "age" => Actual::Text(age.clone()),
                _ => Actual::Missing,
            }
        };
        assert!(first_failing(&rules, &user("UG", "20")).is_none());
        assert_eq!(
            first_failing(&rules, &user("RW", "20")).unwrap().field,
            "country"
        );
        assert_eq!(
            first_failing(&rules, &user("UG", "16")).unwrap().field,
            "age"
        );
        assert_eq!(
            first_failing(&rules, &user("RW", "16")).unwrap().field,
            "country"
        );
        assert!(
            first_failing(&[], &user("RW", "1")).is_none(),
            "no rules, no restriction"
        );
    }

    #[test]
    fn a_broken_stored_rule_denies() {
        let rules = vec![rule("country", "teleport", None)];
        let get = |_: &str| text("UG");
        assert!(first_failing(&rules, &get).is_some());
    }

    #[test]
    fn validation_of_new_rules() {
        let attrs: HashSet<String> = ["member".to_string()].into();
        let ok = |r: ClientAccessRule| validate(&[r], &attrs);

        assert!(ok(rule("country", "in", Some(json!(["UG"])))).is_ok());
        assert!(ok(rule("age", "gte", Some(json!(18)))).is_ok());
        assert!(ok(rule("age", "gte", Some(json!("18")))).is_ok());
        assert!(ok(rule("attr:member", "is_set", None)).is_ok());
        assert!(ok(rule("attr:member", "is_set", Some(Value::Null))).is_ok());

        assert!(
            ok(rule("nope", "eq", Some(json!("x")))).is_err(),
            "unknown field"
        );
        assert!(
            ok(rule("attr:nope", "eq", Some(json!("x")))).is_err(),
            "unknown attribute"
        );
        assert!(
            ok(rule("country", "teleport", Some(json!("x")))).is_err(),
            "unknown op"
        );
        assert!(
            ok(rule("country", "in", Some(json!("UG")))).is_err(),
            "in needs a list"
        );
        assert!(
            ok(rule("country", "in", Some(json!([])))).is_err(),
            "empty list"
        );
        assert!(
            ok(rule("country", "in", Some(json!([["UG"]])))).is_err(),
            "nested list"
        );
        assert!(
            ok(rule("country", "eq", Some(json!(["UG"])))).is_err(),
            "eq takes one value"
        );
        assert!(
            ok(rule("age", "gte", Some(json!("old")))).is_err(),
            "not a number"
        );
        assert!(ok(rule("age", "gte", None)).is_err(), "needs a value");
        assert!(
            ok(rule("country", "is_set", Some(json!("x")))).is_err(),
            "is_set takes none"
        );
        assert!(ok(rule("age", "between", Some(json!([18, 65])))).is_ok());
        assert!(ok(rule("age", "between", Some(json!(["18", "65"])))).is_ok());
        assert!(
            ok(rule("age", "between", Some(json!([65, 18])))).is_err(),
            "lowest first"
        );
        assert!(
            ok(rule("age", "between", Some(json!([18])))).is_err(),
            "two values"
        );
        assert!(
            ok(rule("age", "between", Some(json!([18, 30, 65])))).is_err(),
            "two values"
        );
        assert!(
            ok(rule("age", "between", Some(json!([18, "old"])))).is_err(),
            "numbers"
        );
        assert!(
            ok(rule("age", "between", Some(json!(18)))).is_err(),
            "needs a list"
        );
        assert!(ok(rule("age", "between", None)).is_err(), "needs a value");
        assert!(
            ok(rule("country", "eq", Some(json!("x".repeat(300))))).is_err(),
            "too long"
        );

        let many: Vec<_> = (0..=MAX_RULES)
            .map(|_| rule("country", "is_set", None))
            .collect();
        assert!(validate(&many, &attrs).is_err(), "too many rules");
        assert!(validate(&many[..MAX_RULES], &attrs).is_ok());
    }

    #[test]
    fn values_of_attributes_from_json() {
        assert_eq!(actual_from_json(br#""abc""#), text("abc"));
        assert_eq!(actual_from_json(b"42"), text("42"));
        assert_eq!(actual_from_json(b"true"), Actual::Bool(true));
        assert_eq!(
            actual_from_json(br#"["a", 1, false, {"x": 1}]"#),
            Actual::List(vec!["a".into(), "1".into(), "false".into()])
        );
        assert_eq!(actual_from_json(b"null"), Actual::Missing);
        assert_eq!(actual_from_json(b"{}"), Actual::Missing);
        assert_eq!(actual_from_json(b"not json"), Actual::Missing);
    }
}

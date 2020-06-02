use once_cell::sync::Lazy;
use regex::Regex;

const USERNAME_MIN: usize = 5;
const EMAIL_MIN: usize = 3;
const PASSWORD_MIN: usize = 6;

static EMAIL_USER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?i)[a-z0-9.!#$%&'*+/=?^_`{|}~-]+\z").unwrap());

static EMAIL_DOMAIN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)*$",
    )
    .unwrap()
});

pub fn validate_email(email_str_vec: Vec<&str>) -> bool {
    let domain_part = email_str_vec[0];
    let user_part = email_str_vec[1];
    if !validate_length(domain_part.len(), EMAIL_MIN)
        || !validate_length(user_part.len(), EMAIL_MIN)
    {
        return false;
    }
    if !EMAIL_USER_RE.is_match(user_part) {
        return false;
    }

    EMAIL_DOMAIN_RE.is_match(domain_part)
}

// need to improve validation with regex
pub fn validate_username(username: &str) -> bool {
    username.len() >= USERNAME_MIN
}

pub fn validate_password(password: &str) -> bool {
    password.len() >= PASSWORD_MIN
}

fn validate_length(len: usize, min: usize) -> bool {
    len >= min
}

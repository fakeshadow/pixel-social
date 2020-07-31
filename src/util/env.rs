use std::env::var;

#[derive(Clone)]
pub struct Env {
    postgres_url: String,
    redis_url: String,
    server_ip: String,
    server_port: String,
    cors_origin: String,
    use_mail: bool,
    use_sms: bool,
    use_rep: bool,
}

impl Env {
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();

        let postgres_url = var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
        let redis_url = var("REDIS_URL").expect("REDIS_URL must be set in .env");
        let server_ip = var("SERVER_IP").unwrap_or_else(|_| "127.0.0.1".to_owned());
        let server_port = var("SERVER_PORT").unwrap_or_else(|_| "8080".to_owned());
        let cors_origin = var("CORS_ORIGIN").unwrap_or_else(|_| "All".to_owned());
        let use_mail = var("USE_MAIL")
            .unwrap_or_else(|_| "true".to_owned())
            .parse::<bool>()
            .unwrap_or(true);
        let use_sms = var("USE_SMS")
            .unwrap_or_else(|_| "false".to_owned())
            .parse::<bool>()
            .unwrap_or(false);
        let use_rep = var("USE_REPORT")
            .unwrap_or_else(|_| "false".to_owned())
            .parse::<bool>()
            .unwrap_or(false);

        Self {
            postgres_url,
            redis_url,
            server_ip,
            server_port,
            cors_origin,
            use_mail,
            use_sms,
            use_rep,
        }
    }

    pub fn postgres_url(&self) -> &str {
        &self.postgres_url
    }

    pub fn redis_url(&self) -> &str {
        &self.redis_url
    }

    pub fn use_mail(&self) -> bool {
        self.use_mail
    }

    pub fn use_sms(&self) -> bool {
        self.use_sms
    }

    pub fn use_rep(&self) -> bool {
        self.use_rep
    }

    pub fn cors_origin(&self) -> &str {
        &self.cors_origin
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.server_ip, self.server_port)
    }
}

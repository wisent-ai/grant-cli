//! Where grant-cli's data lives: the fleet database `grant-cli`. Stado names
//! the Skarbiec item that holds its address (`stado database resolve`), and
//! Skarbiec answers the pooler URL and the provider's root certificate to
//! the consumer `grant-cli-database-client`, whose bearer Stado keeps in
//! `~/.stado/grant-cli-database-client-skarbiec-token`. Every refusal names
//! the step that failed and what it answered.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow};
use postgres::Client;
use postgres::config::SslMode;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

const DATABASE: &str = "grant-cli";
/// Who asks Stado's directory; the database lists `grant-cli` as consumer.
const DIRECTORY_CONSUMER: &str = "grant-cli";
/// Who reads the credential item; it may read exactly the two fields below.
const CREDENTIAL_CONSUMER: &str = "grant-cli-database-client";
const TOKEN_FILE: &str = "grant-cli-database-client-skarbiec-token";

#[derive(Deserialize)]
struct Resolution {
    credential_item: String,
}

#[derive(Deserialize)]
struct Route {
    url: String,
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

fn stado() -> Result<PathBuf> {
    let stado = home().join(".stado/bin/stado");
    if stado.is_file() {
        Ok(stado)
    } else {
        Err(anyhow!("Stado is not installed at {}; grant-cli finds its database through Stado", stado.display()))
    }
}

/// `stado <arguments>`, with exactly `environment` when one is given, so no
/// ambient variable selects the identity a credential read runs under.
fn run(arguments: &[&str], environment: Option<&[(&str, String)]>) -> Result<String> {
    let operation = format!("stado {}", arguments.join(" "));
    let mut command = Command::new(stado()?);
    command.args(arguments).stdin(Stdio::null());
    if let Some(environment) = environment {
        command.env_clear();
        for (name, value) in environment {
            command.env(name, value);
        }
    }
    let output = command.output().with_context(|| format!("{operation} could not start"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("{operation} exited {}: {}", output.status, detail.trim()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn answer<T: DeserializeOwned>(arguments: &[&str]) -> Result<T> {
    let output = run(arguments, None)?;
    serde_json::from_str(&output)
        .with_context(|| format!("stado {} answered unreadable JSON", arguments.join(" ")))
}

/// A value answered as a JSON string, as `{"value": …}`, or as text.
fn decoded(output: &str) -> Option<String> {
    let value = match serde_json::from_str::<Value>(output) {
        Ok(Value::String(value)) => value,
        Ok(Value::Object(object)) => object.get("value")?.as_str()?.to_owned(),
        Ok(_) => return None,
        Err(_) => output.to_owned(),
    };
    let value = value.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

fn read_field(route: &str, item: &str, field: &str) -> Result<String> {
    let environment = [
        ("HOME", home().display().to_string()),
        ("PATH", std::env::var("PATH").unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin".into())),
        ("TMPDIR", std::env::temp_dir().display().to_string()),
        ("STADO_CREDENTIALS_ADMIN_URL", route.to_owned()),
        ("STADO_CREDENTIALS_ADMIN_CONSUMER", CREDENTIAL_CONSUMER.to_owned()),
        ("STADO_CREDENTIALS_ADMIN_TOKEN_FILE", home().join(".stado").join(TOKEN_FILE).display().to_string()),
    ];
    let output = run(&["secrets", "get", item, "--field", field], Some(&environment))?;
    decoded(&output).ok_or_else(|| {
        anyhow!("stado secrets get {item} --field {field} as {CREDENTIAL_CONSUMER} answered an empty value")
    })
}

/// A client of the fleet database, over TLS verified against the provider
/// root certificate the credential item carries.
pub fn connect() -> Result<Client> {
    let resolution: Resolution =
        answer(&["database", "resolve", DATABASE, "--consumer", DIRECTORY_CONSUMER, "--json"])?;
    let route: Route =
        answer(&["service", "directory", "connect", "skarbiec", "--consumer", DIRECTORY_CONSUMER, "--json"])?;
    let item = resolution.credential_item;
    let url = read_field(&route.url, &item, "pooler_url")?;
    let certificate = read_field(&route.url, &item, "ca_certificate")?;
    let certificate = native_tls::Certificate::from_pem(certificate.as_bytes())
        .with_context(|| format!("{item}#ca_certificate is not a PEM certificate"))?;
    let tls = native_tls::TlsConnector::builder()
        .add_root_certificate(certificate)
        .build()
        .context("the TLS connector for the fleet database could not be built")?;
    let mut config: postgres::Config =
        url.parse().with_context(|| format!("{item}#pooler_url is not a Postgres connection URL"))?;
    config.ssl_mode(SslMode::Require);
    config
        .connect(postgres_native_tls::MakeTlsConnector::new(tls))
        .with_context(|| format!("connecting to the fleet database {DATABASE} through {item}#pooler_url failed"))
}

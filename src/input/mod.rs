use clap::{App, Arg};
use std::str::FromStr;

pub fn parse_user_input() -> UserArgs {
    let matches = App::new("KubEx - Kubernetes Explorer")
        .version("0.1.0")
        .author("Pavel Pscheidl <pavelpscheidl@gmail.com>")
        .about("Discovers unused ConfigMaps and Secrets")
        .arg(
            Arg::with_name("KUBECONFIG")
                .short("k")
                .long("kubeconfig")
                .value_name("PATH_TO_KUBECONFIG")
                .help("Path to a KUBECONFIG file. When not set, env is used.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("NAMESPACE")
                .short("n")
                .long("namespace")
                .value_name("NAMESPACE")
                .help("Namespace to search in.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .short("o")
                .long("output")
                .value_name("OUTPUT")
                .help("Output format. YAML by default.")
                .possible_values(&["yaml", "json"])
                .default_value("yaml")
                .takes_value(true),
        )
        .get_matches();

    UserArgs::new(
        matches.value_of("NAMESPACE").map(|arg| arg.to_string()),
        matches.value_of("NAMESPACE").map(|arg| arg.to_string()),
        matches.value_of("OUTPUT").map_or(Output::Yaml, |arg| {
            Output::from_str(arg).unwrap_or(Output::Yaml)
        }),
    )
}

pub struct UserArgs {
    pub kubeconfig: Option<String>,
    pub namespace: Option<String>,
    pub output: Output,
}

impl UserArgs {
    pub fn new(kubeconfig: Option<String>, namespace: Option<String>, output: Output) -> Self {
        UserArgs {
            kubeconfig,
            namespace,
            output,
        }
    }
}

pub enum Output {
    Yaml,
    Json,
}

impl FromStr for Output {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return match s.trim().to_lowercase().as_str() {
            "yaml" => Ok(Output::Yaml),
            "json" => Ok(Output::Json),
            _ => Err("Invalid output format".to_string()),
        };
    }
}

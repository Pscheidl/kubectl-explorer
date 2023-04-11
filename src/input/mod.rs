use clap::{builder::PossibleValuesParser, Arg, ArgAction, Command};
use std::str::FromStr;

pub fn parse_user_input() -> UserArgs {
    let matches = Command::new("KubEx - Kubernetes Explorer")
        .version("0.1.0")
        .author("Pavel Pscheidl <pavelpscheidl@gmail.com>")
        .about("Discovers unused ConfigMaps and Secrets")
        .arg(
            Arg::new("KUBECONFIG")
                .short('k')
                .long("kubeconfig")
                .value_name("PATH_TO_KUBECONFIG")
                .help("Path to a KUBECONFIG file. When not set, env is used.")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("NAMESPACE")
                .short('n')
                .long("namespace")
                .value_name("NAMESPACE")
                .help("Namespace to search in.")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("OUTPUT")
                .short('o')
                .long("output")
                .value_name("OUTPUT")
                .help("Output format. YAML by default.")
                .value_parser(PossibleValuesParser::new(["yaml", "json"]))
                .default_value("yaml")
                .action(ArgAction::Set),
        )
        .get_matches();

    UserArgs::new(
        matches
            .get_one::<String>("KUBECONFIG")
            .map(|arg| arg.to_string()),
        matches
            .get_one::<String>("NAMESPACE")
            .map(|arg| arg.to_string()),
        matches
            .get_one::<String>("OUTPUT")
            .map_or(Output::Yaml, |arg| {
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

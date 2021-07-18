use clap::{App, Arg};

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
        .get_matches();

    UserArgs::new(
        matches
            .value_of("KUBECONFIG")
            .map_or(None, |f| Some(f.to_string())),
        matches
            .value_of("NAMESPACE")
            .map_or(None, |f| Some(f.to_string())),
    )
}

pub struct UserArgs {
    pub kubeconfig: Option<String>,
    pub namespace: Option<String>,
}

impl UserArgs {
    pub fn new(kubeconfig: Option<String>, namespace: Option<String>) -> Self {
        UserArgs {
            kubeconfig,
            namespace,
        }
    }
}

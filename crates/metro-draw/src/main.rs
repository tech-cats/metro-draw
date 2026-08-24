use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{ArgAction, Parser, Subcommand};
use metro_draw::MetroMap;
use thiserror::Error;

#[derive(Debug, Parser)]
#[command(name = "mtrd", version, about = "Work with metro map manifests")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Convert a metro map between YAML and JSON.
    Convert {
        /// Source .yaml, .yml, or .json file.
        input: PathBuf,

        /// Destination file; its extension selects the output format.
        output: PathBuf,
    },

    /// Check whether a metro map has valid syntax and schema.
    Check {
        /// Print canonical YAML (-v) or detailed debug output (-vv).
        #[arg(short = 'v', action = ArgAction::Count)]
        verbose: u8,

        /// Metro map file to check.
        input: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Yaml,
    Json,
}

impl Format {
    fn from_path(path: &Path) -> Result<Self, CliError> {
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);

        match extension.as_deref() {
            Some("yaml" | "yml") => Ok(Self::Yaml),
            Some("json") => Ok(Self::Json),
            _ => Err(CliError::UnsupportedFormat(path.to_path_buf())),
        }
    }
}

#[derive(Debug, Error)]
enum CliError {
    #[error("cannot determine format for '{0}'; use a .yaml, .yml, or .json extension")]
    UnsupportedFormat(PathBuf),

    #[error("input and output formats are both {0}")]
    SameFormat(&'static str),

    #[error("failed to read '{path}': {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write '{path}': {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid YAML in '{path}': {source}")]
    ParseYaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("invalid JSON in '{path}': {source}")]
    ParseJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to serialize YAML: {0}")]
    SerializeYaml(#[source] serde_yaml::Error),

    #[error("failed to serialize JSON: {0}")]
    SerializeJson(#[source] serde_json::Error),

    #[error("verbosity may be specified at most twice")]
    ExcessiveVerbosity,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(output) => {
            print_output(&output);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<String, CliError> {
    match cli.command {
        Command::Convert { input, output } => {
            convert(&input, &output)?;
            Ok(format!("{} -> {}", input.display(), output.display()))
        }
        Command::Check { input, verbose } => check(&input, verbose),
    }
}

fn convert(input: &Path, output: &Path) -> Result<(), CliError> {
    let input_format = Format::from_path(input)?;
    let output_format = Format::from_path(output)?;
    if input_format == output_format {
        return Err(CliError::SameFormat(match input_format {
            Format::Yaml => "YAML",
            Format::Json => "JSON",
        }));
    }

    let map = read_map(input, input_format)?;
    let mut encoded = serialize_map(&map, output_format)?;
    if !encoded.ends_with('\n') {
        encoded.push('\n');
    }

    fs::write(output, encoded).map_err(|source| CliError::Write {
        path: output.to_path_buf(),
        source,
    })
}

fn check(input: &Path, verbose: u8) -> Result<String, CliError> {
    let format = Format::from_path(input)?;
    let map = read_map(input, format)?;

    match verbose {
        0 => Ok(format!("{}: valid", input.display())),
        1 => map.to_yaml().map_err(CliError::SerializeYaml),
        2 => Ok(format!("{map:#?}")),
        _ => Err(CliError::ExcessiveVerbosity),
    }
}

fn read_map(path: &Path, format: Format) -> Result<MetroMap, CliError> {
    let contents = fs::read_to_string(path).map_err(|source| CliError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    match format {
        Format::Yaml => MetroMap::from_yaml(&contents).map_err(|source| CliError::ParseYaml {
            path: path.to_path_buf(),
            source,
        }),
        Format::Json => MetroMap::from_json(&contents).map_err(|source| CliError::ParseJson {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn serialize_map(map: &MetroMap, format: Format) -> Result<String, CliError> {
    match format {
        Format::Yaml => map.to_yaml().map_err(CliError::SerializeYaml),
        Format::Json => map.to_json().map_err(CliError::SerializeJson),
    }
}

fn print_output(output: &str) {
    print!("{output}");
    if !output.ends_with('\n') {
        println!();
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    const YAML: &str = r#"
stations:
  - id: central
    names:
      en:
        - Central
    position: [1.0, 2.0]
lines:
  - id: red
    names:
      en:
        - Red Line
    color: '#ff0000'
    paths:
      - stations: [central]
        closed: false
"#;

    fn temporary_path(extension: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("mtrd-{}-{nonce}.{extension}", std::process::id()))
    }

    #[test]
    fn parses_cli_contract() {
        let cli = Cli::try_parse_from(["mtrd", "check", "-vv", "map.yaml"]).unwrap();

        assert!(matches!(
            cli.command,
            Command::Check {
                verbose: 2,
                input
            } if input == Path::new("map.yaml")
        ));
    }

    #[test]
    fn converts_yaml_to_json_and_json_to_yaml() {
        let yaml_path = temporary_path("yaml");
        let json_path = temporary_path("json");
        let round_trip_path = temporary_path("yml");
        fs::write(&yaml_path, YAML).unwrap();

        convert(&yaml_path, &json_path).unwrap();
        let json = fs::read_to_string(&json_path).unwrap();
        assert_eq!(
            MetroMap::from_json(&json).unwrap().stations[0].id,
            "central"
        );

        convert(&json_path, &round_trip_path).unwrap();
        let yaml = fs::read_to_string(&round_trip_path).unwrap();
        assert!(yaml.contains("position: [1.0, 2.0]"));

        fs::remove_file(yaml_path).unwrap();
        fs::remove_file(json_path).unwrap();
        fs::remove_file(round_trip_path).unwrap();
    }

    #[test]
    fn check_verbosity_selects_yaml_then_debug() {
        let path = temporary_path("yaml");
        fs::write(&path, YAML).unwrap();

        assert!(check(&path, 0).unwrap().ends_with(": valid"));
        assert!(check(&path, 1).unwrap().starts_with("stations:"));
        assert!(check(&path, 2).unwrap().starts_with("MetroMap {"));
        assert!(matches!(check(&path, 3), Err(CliError::ExcessiveVerbosity)));

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_same_or_unknown_formats() {
        assert!(matches!(
            convert(Path::new("map.yaml"), Path::new("copy.yml")),
            Err(CliError::SameFormat("YAML"))
        ));
        assert!(matches!(
            Format::from_path(Path::new("map.txt")),
            Err(CliError::UnsupportedFormat(_))
        ));
    }
}

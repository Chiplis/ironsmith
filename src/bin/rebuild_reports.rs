use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug)]
struct Args {
    threshold: String,
    dims: usize,
    min_cluster_size: usize,
    limit: Option<usize>,
    cards_path: PathBuf,
    reports_dir: PathBuf,
}

fn usage() {
    eprintln!(
        "Usage: cargo run --no-default-features --bin rebuild_reports -- \\
  [--threshold <float>] [--dims <int>] [--min-cluster-size <int>] [--limit <n>] [--cards <path>] [--reports-dir <path>]"
    );
}

fn parse_args(root_dir: &Path) -> Result<Args, String> {
    let mut threshold = env::var("IRONSMITH_PARSER_SEMANTIC_THRESHOLD")
        .or_else(|_| env::var("IRONSMITH_WASM_SEMANTIC_THRESHOLD"))
        .ok();
    let mut dims = env::var("IRONSMITH_WASM_SEMANTIC_DIMS")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(384usize);
    let mut min_cluster_size = env::var("IRONSMITH_WASM_MIN_CLUSTER_SIZE")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(1usize);
    let mut limit = None;
    let mut cards_path = root_dir.join("cards.json");
    let mut reports_dir = PathBuf::from("/reports");

    let mut iter = env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--threshold" => {
                threshold = Some(
                    iter.next()
                        .ok_or_else(|| "--threshold requires a value".to_string())?,
                );
            }
            "--dims" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--dims requires a value".to_string())?;
                dims = raw
                    .parse::<usize>()
                    .map_err(|err| format!("invalid --dims '{raw}': {err}"))?;
            }
            "--min-cluster-size" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--min-cluster-size requires a value".to_string())?;
                min_cluster_size = raw
                    .parse::<usize>()
                    .map_err(|err| format!("invalid --min-cluster-size '{raw}': {err}"))?;
            }
            "--limit" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--limit requires a value".to_string())?;
                limit = Some(
                    raw.parse::<usize>()
                        .map_err(|err| format!("invalid --limit '{raw}': {err}"))?,
                );
            }
            "--cards" => {
                cards_path = PathBuf::from(
                    iter.next()
                        .ok_or_else(|| "--cards requires a value".to_string())?,
                );
            }
            "--reports-dir" => {
                reports_dir = PathBuf::from(
                    iter.next()
                        .ok_or_else(|| "--reports-dir requires a value".to_string())?,
                );
            }
            "-h" | "--help" => {
                usage();
                std::process::exit(0);
            }
            _ => {
                return Err(format!("unknown argument '{arg}'"));
            }
        }
    }

    let threshold = threshold.ok_or_else(|| {
        "missing threshold: pass --threshold or set IRONSMITH_PARSER_SEMANTIC_THRESHOLD".to_string()
    })?;
    Ok(Args {
        threshold,
        dims,
        min_cluster_size,
        limit,
        cards_path,
        reports_dir,
    })
}

fn now_utc_timestamp() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("date")
        .arg("-u")
        .arg("+%Y%m%dT%H%M%SZ")
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::other("date command failed").into());
    }
    let ts = String::from_utf8(output.stdout)?;
    Ok(ts.trim().to_string())
}

fn resolve_reports_dir(
    preferred: &Path,
    root: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if fs::create_dir_all(preferred).is_ok() {
        return Ok(preferred.to_path_buf());
    }
    let fallback = root.join("reports");
    fs::create_dir_all(&fallback)?;
    eprintln!(
        "[WARN] fallback to {} (could not create {})",
        fallback.display(),
        preferred.display()
    );
    Ok(fallback)
}

fn read_false_positive_set(path: &Path) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(HashSet::new());
    }
    let mut out = HashSet::new();
    let raw = fs::read_to_string(path)?;
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("Name:") {
            let name = rest.trim();
            if !name.is_empty() {
                out.insert(name.to_ascii_lowercase());
            }
            continue;
        }
        if line.contains(':') {
            continue;
        }
        out.insert(line.to_ascii_lowercase());
    }
    Ok(out)
}

fn write_filtered_skip_names(
    mismatch_names_file: &Path,
    false_positive_names_file: &Path,
    out_file: &Path,
) -> Result<usize, Box<dyn std::error::Error>> {
    let excluded = read_false_positive_set(false_positive_names_file)?;
    let raw = fs::read_to_string(mismatch_names_file)?;
    let mut kept = Vec::new();
    for line in raw.lines() {
        let name = line.trim();
        if name.is_empty() {
            continue;
        }
        if !excluded.contains(&name.to_ascii_lowercase()) {
            kept.push(name.to_string());
        }
    }
    fs::write(out_file, kept.join("\n"))?;
    Ok(kept.len())
}

fn run_checked(command: &mut Command, label: &str) -> Result<(), Box<dyn std::error::Error>> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("{label} failed with status {status}")).into())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let args = parse_args(&root_dir).map_err(std::io::Error::other)?;
    let reports_dir = resolve_reports_dir(&args.reports_dir, &root_dir)?;

    let false_positives_file = root_dir
        .join("scripts")
        .join("semantic_false_positives.txt");
    let timestamp = now_utc_timestamp()?;
    let safe_threshold = args.threshold.replace('.', "_");
    let run_id = format!("{safe_threshold}_{}_{}", args.dims, timestamp);
    let tmp_dir = env::temp_dir();

    let mismatch_names_file = tmp_dir.join(format!("ironsmith_wasm_mismatch_names_{run_id}.txt"));
    let filtered_skip_names_file = reports_dir.join(format!("ironsmith_wasm_skip_names_{run_id}.txt"));
    let failures_report = tmp_dir.join(format!("ironsmith_wasm_threshold_failures_{run_id}.json"));
    let cluster_report = tmp_dir.join(format!("ironsmith_wasm_cluster_report_{run_id}.json"));
    let mismatch_report = reports_dir.join(format!(
        "ironsmith_wasm_semantic_mismatch_report_{run_id}.json"
    ));
    let unparsable_report =
        reports_dir.join(format!("ironsmith_wasm_unparsable_clusters_{run_id}.json"));

    eprintln!(
        "[INFO] computing semantic threshold failures (threshold={}, dims={})...",
        args.threshold, args.dims
    );
    let mut audit_cmd = Command::new("cargo");
    audit_cmd
        .current_dir(&root_dir)
        .arg("run")
        .arg("--quiet")
        .arg("--no-default-features")
        .arg("--bin")
        .arg("audit_oracle_clusters")
        .arg("--")
        .arg("--cards")
        .arg(&args.cards_path)
        .arg("--use-embeddings")
        .arg("--embedding-dims")
        .arg(args.dims.to_string())
        .arg("--embedding-threshold")
        .arg(&args.threshold)
        .arg("--min-cluster-size")
        .arg(args.min_cluster_size.to_string())
        .arg("--top-clusters")
        .arg("20000")
        .arg("--examples")
        .arg("1")
        .arg("--mismatch-names-out")
        .arg(&mismatch_names_file)
        .arg("--failures-out")
        .arg(&failures_report)
        .arg("--json-out")
        .arg(&cluster_report);
    if let Some(limit) = args.limit {
        audit_cmd.arg("--limit").arg(limit.to_string());
    }
    if false_positives_file.exists() {
        audit_cmd
            .arg("--false-positive-names")
            .arg(&false_positives_file);
    }
    run_checked(&mut audit_cmd, "audit_oracle_clusters")?;

    let excluded_count = write_filtered_skip_names(
        &mismatch_names_file,
        &false_positives_file,
        &filtered_skip_names_file,
    )?;
    fs::copy(&failures_report, &mismatch_report)?;

    let jq_expr = r#"
{
  generated_at: $ts,
  threshold: $threshold,
  embedding_dims: $dims,
  cards_processed: .cards_processed,
  semantic_failures_report: .failures,
  parse_failures: .parse_failures,
  unparsable_clusters_by_error: (
    .clusters
    | map(
      select(.parse_failures > 0)
      | {
          error: (.top_errors[0].error // "unclassified"),
          cluster: {
            signature: .signature,
            size: .size,
            parse_failures: .parse_failures,
            parse_failure_rate: .parse_failure_rate,
            top_errors: .top_errors,
            examples: .examples
          }
        }
    )
    | if length == 0 then
        {}
      else
        sort_by(.error)
        | group_by(.error)
        | map(
          {
            (.[0].error): {
              cluster_count: length,
              total_parse_failures: (map(.cluster.parse_failures) | add),
              clusters: (map(.cluster))
            }
          }
        )
        | add
      end
  )
}
"#;
    let unparsable_handle = fs::File::create(&unparsable_report)?;
    let mut jq_cmd = Command::new("jq");
    jq_cmd
        .arg("--arg")
        .arg("ts")
        .arg(&timestamp)
        .arg("--arg")
        .arg("threshold")
        .arg(&args.threshold)
        .arg("--arg")
        .arg("dims")
        .arg(args.dims.to_string())
        .arg(jq_expr)
        .arg(&cluster_report)
        .stdout(Stdio::from(unparsable_handle));
    run_checked(&mut jq_cmd, "jq report transform")?;

    eprintln!(
        "[INFO] semantic gating active: excluding {} below-threshold card(s)",
        excluded_count
    );
    eprintln!(
        "[INFO] semantic mismatch report: {}",
        mismatch_report.display()
    );
    eprintln!(
        "[INFO] unparsable cluster report: {}",
        unparsable_report.display()
    );
    eprintln!(
        "[INFO] generated skip names file: {}",
        filtered_skip_names_file.display()
    );

    Ok(())
}

use std::{
    env,
    path::{Path, PathBuf},
};

use serde::Serialize;

#[derive(Serialize)]
struct RuntimeHealthSnapshot {
    data_dir: String,
    resources_dir: Option<String>,
    managed_pyannote_runtime_dir: String,
    managed_pyannote_python_dir: String,
    managed_pyannote_model_dir: String,
    health: sbobino_infrastructure::RuntimeHealth,
}

fn usage() -> ! {
    eprintln!(
        "Usage: runtime_health_snapshot --data-dir <path> [--resources-dir <path>] [--pretty]"
    );
    std::process::exit(1);
}

fn next_arg<I>(args: &mut I) -> String
where
    I: Iterator<Item = String>,
{
    args.next().unwrap_or_else(|| usage())
}

fn as_display_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mut data_dir: Option<PathBuf> = None;
    let mut resources_dir: Option<PathBuf> = None;
    let mut pretty = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data-dir" => data_dir = Some(PathBuf::from(next_arg(&mut args))),
            "--resources-dir" => resources_dir = Some(PathBuf::from(next_arg(&mut args))),
            "--pretty" => pretty = true,
            _ => usage(),
        }
    }

    let data_dir = data_dir.unwrap_or_else(|| usage());
    let bundle = sbobino_infrastructure::bootstrap(&data_dir, resources_dir.clone())?;
    let health = bundle.runtime_factory.runtime_health()?;

    let snapshot = RuntimeHealthSnapshot {
        data_dir: as_display_string(&data_dir),
        resources_dir: resources_dir.as_ref().map(|path| as_display_string(path)),
        managed_pyannote_runtime_dir: as_display_string(
            &bundle.runtime_factory.managed_pyannote_runtime_dir(),
        ),
        managed_pyannote_python_dir: as_display_string(
            &bundle.runtime_factory.managed_pyannote_python_dir(),
        ),
        managed_pyannote_model_dir: as_display_string(
            &bundle.runtime_factory.managed_pyannote_model_dir(),
        ),
        health,
    };

    let body = if pretty {
        serde_json::to_string_pretty(&snapshot)
    } else {
        serde_json::to_string(&snapshot)
    }
    .map_err(|error| format!("failed to serialize runtime health snapshot: {error}"))?;

    println!("{body}");
    Ok(())
}

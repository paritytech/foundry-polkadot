use std::{collections::BTreeMap, fs, path::Path, process::Command};

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

fn is_elf(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[..4] == ELF_MAGIC
}

pub fn resolc_link(
    resolc_path: &Path,
    solc_path: Option<&Path>,
    bytecodes: &BTreeMap<String, Vec<u8>>,
    libraries: &[String],
) -> eyre::Result<BTreeMap<String, Vec<u8>>> {
    let temp_dir = tempfile::tempdir()?;

    let mut relative_paths = Vec::new();
    let mut path_to_id: BTreeMap<std::path::PathBuf, String> = BTreeMap::new();
    for (contract_id, bytecode) in bytecodes {
        let relative = std::path::PathBuf::from(format!("{contract_id}.pvm"));
        let absolute = temp_dir.path().join(&relative);
        if let Some(parent) = absolute.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&absolute, bytecode)?;
        path_to_id.insert(relative.clone(), contract_id.clone());
        relative_paths.push(relative);
    }

    let mut cmd = Command::new(resolc_path);
    cmd.arg("--link");
    cmd.current_dir(temp_dir.path());
    if let Some(solc) = solc_path {
        cmd.arg("--solc").arg(solc);
    }
    for lib in libraries {
        cmd.arg("--libraries").arg(lib);
    }
    for path in &relative_paths {
        cmd.arg(path);
    }

    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eyre::bail!("resolc --link failed (exit code {:?}): {stderr}", output.status.code());
    }

    let mut linked = BTreeMap::new();
    let mut still_unlinked = Vec::new();
    for relative in &relative_paths {
        let contract_id = &path_to_id[relative];
        let original = &bytecodes[contract_id];
        let absolute = temp_dir.path().join(relative);
        let updated = fs::read(&absolute)?;

        if is_elf(original) && !is_elf(&updated) {
            linked.insert(contract_id.clone(), updated);
        } else if is_elf(original) && is_elf(&updated) {
            still_unlinked.push(contract_id.clone());
        }
    }

    if !still_unlinked.is_empty() {
        tracing::warn!(
            target: "forge",
            "resolc --link: {} contract(s) remain unlinked: {}",
            still_unlinked.len(),
            still_unlinked.join(", ")
        );
    }

    Ok(linked)
}

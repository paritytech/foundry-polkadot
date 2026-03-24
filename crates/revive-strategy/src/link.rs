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

    let mut file_paths = Vec::new();
    let mut path_to_id: BTreeMap<std::path::PathBuf, String> = BTreeMap::new();
    for (contract_id, bytecode) in bytecodes {
        let safe_name = contract_id.replace(['/', '\\'], "_").replace(':', "_");
        let file_path = temp_dir.path().join(format!("{safe_name}.pvm"));
        fs::write(&file_path, bytecode)?;
        path_to_id.insert(file_path.clone(), contract_id.clone());
        file_paths.push(file_path);
    }

    let mut cmd = Command::new(resolc_path);
    cmd.arg("--link");
    if let Some(solc) = solc_path {
        cmd.arg("--solc").arg(solc);
    }
    for lib in libraries {
        cmd.arg("--libraries").arg(lib);
    }
    for path in &file_paths {
        cmd.arg(path);
    }

    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eyre::bail!("resolc --link failed (exit code {:?}): {stderr}", output.status.code());
    }

    let mut linked = BTreeMap::new();
    for file_path in &file_paths {
        let contract_id = &path_to_id[file_path];
        let original = &bytecodes[contract_id];
        let updated = fs::read(file_path)?;

        if is_elf(original) && !is_elf(&updated) {
            linked.insert(contract_id.clone(), updated);
        } else if is_elf(original) && is_elf(&updated) {
            tracing::debug!(
                target: "forge",
                "resolc --link: {contract_id} remains unlinked (still ELF)"
            );
        }
    }

    Ok(linked)
}

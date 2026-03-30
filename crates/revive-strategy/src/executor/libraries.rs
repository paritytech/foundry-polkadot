use alloy_primitives::{hex, keccak256};
use foundry_compilers::{
    Artifact,
    artifacts::{BytecodeObject, Libraries, ObjectFormat},
    compilers::resolc::dual_compiled_contracts::{DualCompiledContract, DualCompiledContracts},
    contracts::ArtifactContracts,
    info::ContractInfo,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use super::context::ReviveExecutorStrategyContext;

fn matches_by_name(info: &ContractInfo, name: &str, source: &Path) -> bool {
    if info.name != name {
        return false;
    }
    let info_path = match info.path.as_ref() {
        Some(p) => Path::new(p),
        None => return false,
    };
    info_path == source || source.ends_with(info_path) || info_path.ends_with(source)
}

pub fn link_libraries(
    ctx: &mut ReviveExecutorStrategyContext,
    config: &foundry_config::Config,
    root: &Path,
    linked_contracts: &ArtifactContracts,
    libraries: &Libraries,
) -> eyre::Result<()> {
    if ctx.dual_compiled_contracts.is_empty() || libraries.libs.is_empty() {
        return Ok(());
    }

    let contracts_needing_linking = collect_contracts_needing_linking(ctx, root);
    if contracts_needing_linking.is_empty() {
        return Ok(());
    }

    add_missing_dcc_entries(ctx, root, &contracts_needing_linking);
    patch_library_guards(&mut ctx.dual_compiled_contracts, libraries);
    update_evm_bytecodes(
        &mut ctx.dual_compiled_contracts,
        linked_contracts,
        &contracts_needing_linking,
    );

    link_pvm_bytecodes(
        &mut ctx.dual_compiled_contracts,
        config,
        root,
        libraries,
        &contracts_needing_linking,
    )
}

/// Adds DCC entries for contracts present in resolc output but missing from DCC
/// (e.g. libraries without factory_dependencies that DualCompiledContracts::new() skips).
fn add_missing_dcc_entries(
    ctx: &mut ReviveExecutorStrategyContext,
    root: &Path,
    contracts_needing_linking: &BTreeSet<String>,
) {
    let Some(resolc_output) = ctx.resolc_output.as_ref() else { return };

    let entries: Vec<_> = resolc_output
        .artifact_ids()
        .filter_map(|(resolc_id, resolc_artifact)| {
            let source = resolc_id.source.strip_prefix(root).unwrap_or(&resolc_id.source);
            let key = format!("{}:{}", source.display(), resolc_id.name);
            if !contracts_needing_linking.contains(&key) {
                return None;
            }
            if ctx
                .dual_compiled_contracts
                .iter()
                .any(|(info, _)| matches_by_name(info, &resolc_id.name, &resolc_id.source))
            {
                return None;
            }

            let resolc_bytecode =
                resolc_artifact.get_bytecode().map(|b| b.object.clone()).unwrap_or_default();
            if resolc_bytecode.as_bytes().is_none_or(|b| b.is_empty()) {
                return None;
            }

            let resolc_extras = resolc_artifact.extensions.resolc_extras();
            let resolc_hash =
                resolc_extras.as_ref().and_then(|x| x.hash.clone()).unwrap_or_default();

            let mut factory_deps: Vec<BytecodeObject> = resolc_extras
                .as_ref()
                .and_then(|x| x.factory_dependencies.as_ref())
                .map(|deps_map| {
                    deps_map
                        .keys()
                        .filter_map(|hash| {
                            resolc_output.artifact_ids().find_map(|(_, art)| {
                                let h = art.extensions.resolc_extras().and_then(|e| e.hash)?;
                                if &h == hash {
                                    art.get_bytecode().map(|b| b.object.clone())
                                } else {
                                    None
                                }
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            if let Some(unlinked_deps) =
                resolc_extras.as_ref().and_then(|x| x.factory_dependencies_unlinked.as_ref())
            {
                for dep_id in unlinked_deps {
                    let parts: Vec<&str> = dep_id.rsplitn(2, ':').collect();
                    if parts.len() == 2 {
                        let (dep_name, dep_source) = (parts[0], parts[1]);
                        if let Some(bytecode) =
                            resolc_output.artifact_ids().find_map(|(id, art)| {
                                if id.name == dep_name
                                    && id.source.to_string_lossy().ends_with(dep_source)
                                {
                                    art.get_bytecode().map(|b| b.object.clone())
                                } else {
                                    None
                                }
                            })
                        {
                            factory_deps.push(bytecode);
                        }
                    }
                }
            }

            let (evm_bytecode, evm_deployed, evm_hash) = ctx
                .compilation_output
                .as_ref()
                .and_then(|solc_output| {
                    solc_output
                        .artifact_ids()
                        .map(|(id, v)| (id.with_stripped_file_prefixes(root), v))
                        .find(|(id, _)| {
                            id.name == resolc_id.name
                                && (id.source == resolc_id.source
                                    || id.source.file_name() == resolc_id.source.file_name())
                        })
                })
                .map(|(_, solc_artifact)| {
                    let bc = solc_artifact
                        .get_bytecode_bytes()
                        .map(|b| BytecodeObject::Bytecode(b.into_owned()))
                        .unwrap_or_default();
                    let deployed = solc_artifact
                        .get_deployed_bytecode()
                        .and_then(|db| db.bytecode.as_ref().map(|b| b.object.clone()))
                        .unwrap_or_default();
                    let hash = deployed.as_bytes().map(keccak256).unwrap_or_default();
                    (bc, deployed, hash)
                })
                .unwrap_or_default();

            let info = ContractInfo {
                name: resolc_id.name.clone(),
                path: Some(resolc_id.source.to_string_lossy().into_owned()),
            };

            Some((
                info,
                DualCompiledContract {
                    resolc_bytecode_hash: resolc_hash,
                    resolc_bytecode: resolc_bytecode.clone(),
                    resolc_deployed_bytecode: resolc_bytecode,
                    resolc_factory_deps: factory_deps,
                    evm_bytecode_hash: evm_hash,
                    evm_bytecode,
                    evm_deployed_bytecode: evm_deployed,
                    evm_immutable_references: None,
                    storage_slots: vec![],
                },
            ))
        })
        .collect();

    for (info, contract) in entries {
        ctx.dual_compiled_contracts.insert(info, contract);
    }
}

/// Patch library self-address guards in DCC deployed bytecodes.
/// Libraries have `PUSH20 <zeros>` at the start of their deployed bytecode; the actual
/// address is filled in at deploy time. We patch it here so migration can match them.
fn patch_library_guards(dcc: &mut DualCompiledContracts, libraries: &Libraries) {
    let lib_addresses: BTreeMap<String, Vec<u8>> = libraries
        .libs
        .iter()
        .flat_map(|(_, libs)| {
            libs.iter().filter_map(|(name, addr)| {
                let addr = addr.strip_prefix("0x").unwrap_or(addr);
                hex::decode(addr).ok().map(|bytes| (name.clone(), bytes))
            })
        })
        .collect();

    let updates: Vec<_> = dcc
        .iter()
        .filter_map(|(info, contract)| {
            let bytes = contract.evm_deployed_bytecode.as_bytes()?;
            if bytes.len() >= 21 && bytes[0] == 0x73 && bytes[1..21].iter().all(|&b| b == 0) {
                if let Some(addr_bytes) = lib_addresses.get(&info.name) {
                    if addr_bytes.len() == 20 {
                        let mut patched = bytes.to_vec();
                        patched[1..21].copy_from_slice(addr_bytes);
                        return Some((info.clone(), patched));
                    }
                }
            }
            None
        })
        .collect();

    for (info, patched) in updates {
        let contract = dcc
            .iter()
            .find(|(i, _)| i.name == info.name && i.path == info.path)
            .map(|(_, c)| c.clone());
        if let Some(mut contract) = contract {
            contract.evm_deployed_bytecode = BytecodeObject::Bytecode(patched.clone().into());
            contract.evm_bytecode_hash = keccak256(&patched);
            dcc.insert(info, contract);
        }
    }
}

fn update_evm_bytecodes(
    dcc: &mut DualCompiledContracts,
    linked_contracts: &ArtifactContracts,
    contracts_needing_linking: &BTreeSet<String>,
) {
    let updates: Vec<_> = linked_contracts
        .iter()
        .filter(|(id, _)| {
            let key = format!("{}:{}", id.source.display(), id.name);
            contracts_needing_linking.iter().any(|c| c.ends_with(&key) || key.ends_with(c))
        })
        .filter_map(|(id, linked)| {
            let (info, mut contract) = dcc
                .iter()
                .find(|(info, _)| matches_by_name(info, &id.name, &id.source))
                .map(|(info, contract)| (info.clone(), contract.clone()))?;

            if let Some(bytecode) = linked.get_bytecode_bytes() {
                contract.evm_bytecode = BytecodeObject::Bytecode(bytecode.into_owned());
            }
            if let Some(deployed) = linked.get_deployed_bytecode()
                && let Some(bytecode_ref) = deployed.bytecode.as_ref()
                && let Some(bytes) = bytecode_ref.object.as_bytes()
            {
                let deployed_bytes = bytes.to_vec();
                contract.evm_deployed_bytecode =
                    BytecodeObject::Bytecode(deployed_bytes.clone().into());
                contract.evm_bytecode_hash = keccak256(&deployed_bytes);
            }

            Some((info, contract))
        })
        .collect();

    for (info, contract) in updates {
        dcc.insert(info, contract);
    }
}

fn collect_contracts_needing_linking(
    ctx: &ReviveExecutorStrategyContext,
    root: &Path,
) -> BTreeSet<String> {
    let Some(resolc_output) = ctx.resolc_output.as_ref() else {
        return BTreeSet::new();
    };

    resolc_output
        .artifact_ids()
        .filter_map(|(id, artifact)| {
            let extras = artifact.extensions.resolc_extras()?;
            let has_missing_libraries =
                extras.missing_libraries.as_ref().is_some_and(|libs| !libs.is_empty());
            let has_unlinked_factory_deps =
                extras.factory_dependencies_unlinked.as_ref().is_some_and(|deps| !deps.is_empty());
            let is_unlinked = extras.object_format == Some(ObjectFormat::ELF);

            if has_missing_libraries || has_unlinked_factory_deps || is_unlinked {
                let source = id.source.strip_prefix(root).unwrap_or(&id.source);
                Some(format!("{}:{}", source.display(), id.name))
            } else {
                None
            }
        })
        .collect()
}

fn link_pvm_bytecodes(
    dcc: &mut DualCompiledContracts,
    config: &foundry_config::Config,
    root: &Path,
    libraries: &Libraries,
    contracts_needing_linking: &BTreeSet<String>,
) -> eyre::Result<()> {
    let resolc =
        config.resolc_compiler().map_err(|e| eyre::eyre!("resolc compiler not found: {e}"))?;

    let solc_path = match &resolc.solc {
        foundry_compilers::compilers::solc::SolcCompiler::Specific(s) => Some(s.solc.clone()),
        foundry_compilers::compilers::solc::SolcCompiler::AutoDetect => {
            use foundry_compilers::compilers::solc::Solc;
            let mut versions = Solc::installed_versions();
            versions.sort();
            versions
                .last()
                .and_then(|v| Solc::find_svm_installed_version(v).ok().flatten().map(|s| s.solc))
        }
    };

    let mut bytecodes = BTreeMap::new();
    for (info, contract) in dcc.iter() {
        if let Some(bytes) = contract.resolc_bytecode.as_bytes() {
            let source = info.path.as_deref().unwrap_or("");
            let source = Path::new(source).strip_prefix(root).unwrap_or(Path::new(source));
            bytecodes.insert(format!("{}:{}", source.display(), info.name), bytes.to_vec());
        }
    }

    if !contracts_needing_linking.iter().any(|c| bytecodes.contains_key(c)) {
        return Ok(());
    }

    let lib_args: Vec<String> = libraries
        .libs
        .iter()
        .flat_map(|(file, libs)| {
            libs.iter().map(move |(name, addr)| format!("{}:{}={}", file.display(), name, addr))
        })
        .collect();

    let linked_pvm =
        crate::link::resolc_link(&resolc.resolc, solc_path.as_deref(), &bytecodes, &lib_args)?;

    let mut old_to_new: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();

    for (contract_id, linked_bytes) in &linked_pvm {
        if let Some(old_bytes) = bytecodes.get(contract_id) {
            old_to_new.insert(old_bytes.clone(), linked_bytes.clone());
        }

        let parts: Vec<&str> = contract_id.rsplitn(2, ':').collect();
        if parts.len() != 2 {
            continue;
        }
        let (contract_name, source_path) = (parts[0], parts[1]);

        let source = Path::new(source_path);
        let matching = dcc
            .iter()
            .find(|(info, _)| {
                info.name == contract_name
                    && info.path.as_ref().is_some_and(|p| {
                        let p = Path::new(p);
                        p == source || p.ends_with(source)
                    })
            })
            .map(|(info, contract)| (info.clone(), contract.clone()));

        if let Some((info, mut contract)) = matching {
            let bytecode_obj = BytecodeObject::Bytecode(linked_bytes.clone().into());
            contract.resolc_bytecode = bytecode_obj.clone();
            contract.resolc_deployed_bytecode = bytecode_obj;
            contract.resolc_bytecode_hash = hex::encode(keccak256(linked_bytes).as_slice());
            dcc.insert(info, contract);
        }
    }

    update_factory_deps(dcc, &old_to_new);

    Ok(())
}

fn update_factory_deps(dcc: &mut DualCompiledContracts, old_to_new: &BTreeMap<Vec<u8>, Vec<u8>>) {
    if old_to_new.is_empty() {
        return;
    }

    let updates: Vec<_> = dcc
        .iter()
        .filter(|(_, contract)| !contract.resolc_factory_deps.is_empty())
        .filter_map(|(info, contract)| {
            let mut changed = false;
            let new_deps: Vec<BytecodeObject> = contract
                .resolc_factory_deps
                .iter()
                .map(|dep| {
                    if let Some(dep_bytes) = dep.as_bytes() {
                        if let Some(new_bytes) = old_to_new.get(dep_bytes.as_ref()) {
                            changed = true;
                            return BytecodeObject::Bytecode(new_bytes.clone().into());
                        }
                    }
                    dep.clone()
                })
                .collect();

            if changed { Some((info.clone(), new_deps)) } else { None }
        })
        .collect();

    for (info, new_deps) in updates {
        let contract = dcc
            .iter()
            .find(|(i, _)| i.name == info.name && i.path == info.path)
            .map(|(_, c)| c.clone());
        if let Some(mut updated) = contract {
            updated.resolc_factory_deps = new_deps;
            dcc.insert(info, updated);
        }
    }
}

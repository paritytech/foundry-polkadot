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
    // Try full path match first, fall back to filename-only match
    info_path == source || info_path.file_name() == source.file_name()
}

pub fn link_libraries(
    ctx: &mut ReviveExecutorStrategyContext,
    config: &foundry_config::Config,
    root: &Path,
    linked_contracts: &ArtifactContracts,
    libraries: &Libraries,
) {
    if ctx.dual_compiled_contracts.is_empty() {
        return;
    }

    add_missing_dcc_entries(ctx, root);
    update_evm_bytecodes(&mut ctx.dual_compiled_contracts, linked_contracts);

    if !libraries.libs.is_empty() {
        let contracts_needing_linking = collect_contracts_needing_linking(ctx);
        if !contracts_needing_linking.is_empty() {
            link_pvm_bytecodes(
                &mut ctx.dual_compiled_contracts,
                config,
                libraries,
                &contracts_needing_linking,
            );
        }
    }
}

/// Adds DCC entries for contracts present in resolc output but missing from DCC
/// (e.g. libraries without factory_dependencies that DualCompiledContracts::new() skips).
fn add_missing_dcc_entries(ctx: &mut ReviveExecutorStrategyContext, root: &Path) {
    let Some(resolc_output) = ctx.resolc_output.as_ref() else { return };

    let entries: Vec<_> = resolc_output
        .artifact_ids()
        .filter_map(|(resolc_id, resolc_artifact)| {
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

            let resolc_hash =
                resolc_artifact.extensions.resolc_extras().and_then(|x| x.hash).unwrap_or_default();

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
                    resolc_factory_deps: vec![],
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

fn update_evm_bytecodes(dcc: &mut DualCompiledContracts, linked_contracts: &ArtifactContracts) {
    let updates: Vec<_> = linked_contracts
        .iter()
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

fn collect_contracts_needing_linking(ctx: &ReviveExecutorStrategyContext) -> BTreeSet<String> {
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
                Some(format!("{}:{}", id.source.to_string_lossy(), id.name))
            } else {
                None
            }
        })
        .collect()
}

fn link_pvm_bytecodes(
    dcc: &mut DualCompiledContracts,
    config: &foundry_config::Config,
    libraries: &Libraries,
    contracts_needing_linking: &BTreeSet<String>,
) {
    let Ok(resolc) = config.resolc_compiler() else {
        tracing::warn!(target: "forge", "resolc compiler not found, skipping PVM linking");
        return;
    };

    let mut bytecodes = BTreeMap::new();
    for (info, contract) in dcc.iter() {
        if let Some(bytes) = contract.resolc_bytecode.as_bytes() {
            let source = info.path.as_deref().unwrap_or("");
            bytecodes.insert(format!("{}:{}", source, info.name), bytes.to_vec());
        }
    }

    if !contracts_needing_linking.iter().any(|c| bytecodes.contains_key(c)) {
        return;
    }

    let lib_args: Vec<String> = libraries
        .libs
        .iter()
        .flat_map(|(file, libs)| {
            libs.iter().map(move |(name, addr)| format!("{}:{}={}", file.display(), name, addr))
        })
        .collect();

    let solc_path = config
        .solc_compiler()
        .ok()
        .and_then(|sc| match sc {
            foundry_compilers::compilers::solc::SolcCompiler::Specific(s) => Some(s),
            _ => None,
        })
        .map(|s| s.solc.clone());

    match crate::link::resolc_link(&resolc.resolc, solc_path.as_deref(), &bytecodes, &lib_args) {
        Ok(linked_pvm) => {
            for (contract_id, linked_bytes) in linked_pvm {
                let parts: Vec<&str> = contract_id.rsplitn(2, ':').collect();
                if parts.len() != 2 {
                    continue;
                }
                let (contract_name, source_path) = (parts[0], parts[1]);

                let matching = dcc
                    .iter()
                    .find(|(info, _)| {
                        info.name == contract_name && info.path.as_deref() == Some(source_path)
                    })
                    .map(|(info, contract)| (info.clone(), contract.clone()));

                if let Some((info, mut contract)) = matching {
                    let bytecode_obj = BytecodeObject::Bytecode(linked_bytes.clone().into());
                    contract.resolc_bytecode = bytecode_obj.clone();
                    contract.resolc_deployed_bytecode = bytecode_obj;
                    contract.resolc_bytecode_hash =
                        hex::encode(keccak256(&linked_bytes).as_slice());
                    dcc.insert(info, contract);
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                target: "forge",
                "resolc --link failed, PVM bytecodes may be unlinked: {e}"
            );
        }
    }
}

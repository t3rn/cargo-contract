// Copyright 2018-2020 Parity Technologies (UK) Ltd.
// This file is part of cargo-contract.
//
// cargo-contract is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// cargo-contract is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with cargo-contract.  If not, see <http://www.gnu.org/licenses/>.

use anyhow::Result;
use subxt::{
    balances::Balances, contracts::*, contracts_gateway::*, runtime_gateway::*, system::System,
    ClientBuilder, ContractsTemplateRuntime,
};
// use self::{deploy::load_contract_code};
use crate::cmd::deploy::load_contract_code;

use crate::{ExtrinsicOpts, HexData};

/// Instantiate a contract stored at the supplied code hash.
/// Returns the account id of the instantiated contract if successful.
///
/// Creates an extrinsic with the `Contracts::instantiate` Call, submits via RPC, then waits for
/// the `ContractsEvent::Instantiated` event.
pub(crate) fn execute_call<'a>(
    extrinsic_opts: &ExtrinsicOpts,
    requester: <ContractsTemplateRuntime as System>::AccountId,
    target_dest: <ContractsTemplateRuntime as System>::AccountId,
    phase: u8,
    code: &'a [u8],
    value: <ContractsTemplateRuntime as Balances>::Balance,
    gas_limit: u64,
    data: HexData,
    // ) -> Result<&'a [u8]> {
) -> Result<()> {
    async_std::task::block_on(async move {
        let cli = ClientBuilder::<ContractsTemplateRuntime>::new()
            .set_url(&extrinsic_opts.url.to_string())
            .build()
            .await?;

        let signer = extrinsic_opts.signer()?;

        let events = cli
            .multistep_call_and_watch(
                &signer,
                requester,
                target_dest,
                phase, // phase = Execution
                &code,
                value,     // value
                gas_limit, // gas_limit
                &data.0,   // input data
            )
            .await?;
        log::info!("multistep_call_and_watch res: {:?}", events);
        let execution_success_event =
            events
                .multistep_execute_phase_success()?
                .ok_or(anyhow::anyhow!(
                    "Failed to find a MultistepExecutePhaseSuccess event"
                ))?;

        //  /Users/macio/projects/substrate.dev/cargo-contract/target/debug/cargo-contract contract call-runtime-gateway --data 00 --suri //Alice --target //Bob --requester //Charlie

        let instantiated = events
            .instantiated()?
            .ok_or(anyhow::anyhow!("Failed to find Instantiated event"))?;


        log::info!(
            "multistep_call_and_watch execution_success_event execution_stamp {:?}",
            execution_success_event.execution_stamp
        );


        Ok(())
    })
}

/// Instantiate a contract stored at the supplied code hash.
/// Returns the account id of the instantiated contract if successful.
///
/// Creates an extrinsic with the `Contracts::instantiate` Call, submits via RPC, then waits for
/// the `ContractsEvent::Instantiated` event.
pub(crate) fn execute_contract_call<'a>(
    extrinsic_opts: &ExtrinsicOpts,
    requester: <ContractsTemplateRuntime as System>::AccountId,
    target_dest: <ContractsTemplateRuntime as System>::AccountId,
    phase: u8,
    code: &'a [u8],
    value: <ContractsTemplateRuntime as Balances>::Balance,
    gas_limit: u64,
    data: HexData,
    // ) -> Result<&'a [u8]> {
) -> Result<()> {
    async_std::task::block_on(async move {
        let cli = ClientBuilder::<ContractsTemplateRuntime>::new()
            .set_url(&extrinsic_opts.url.to_string())
            .build()
            .await?;

        let signer = extrinsic_opts.signer()?;

        let events = cli
            .gateway_contract_exec_and_watch(
                &signer,
                requester,
                target_dest,
                phase, // phase = Execution
                &code,
                value,     // value
                gas_limit, // gas_limit
                &data.0,   // input data
            )
            .await?;
        log::info!("multistep_call_and_watch res: {:?}", events);
        let execution_success_event =
            events
                .multistep_execute_phase_success()?
                .ok_or(anyhow::anyhow!(
                    "Failed to find a MultistepExecutePhaseSuccess event"
                ))?;

        //  /Users/macio/projects/substrate.dev/cargo-contract/target/debug/cargo-contract contract call-runtime-gateway --data 00 --suri //Alice --target //Bob --requester //Charlie

        let instantiated = events
            .instantiated()?
            .ok_or(anyhow::anyhow!("Failed to find Instantiated event"))?;


        log::info!(
            "multistep_call_and_watch execution_success_event execution_stamp {:?}",
            execution_success_event.execution_stamp
        );


        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write};

    use crate::{cmd::deploy::execute_deploy, util::tests::with_tmp_dir, ExtrinsicOpts, HexData};
    use assert_matches::assert_matches;

    const CONTRACT: &str = r#"
(module
    (func (export "call"))
    (func (export "deploy"))
)
"#;

    #[test]
    fn instantiate_contract() {
        with_tmp_dir(|path| {
            let wasm = wabt::wat2wasm(CONTRACT).expect("invalid wabt");

            let wasm_path = path.join("test.wasm");
            let mut file = fs::File::create(&wasm_path).unwrap();
            let _ = file.write_all(&wasm);
            let code = load_contract_code(contract_wasm_path)?;

            let url = url::Url::parse("ws://localhost:9944").unwrap();
            let extrinsic_opts = ExtrinsicOpts {
                url,
                suri: "//Alice".into(),
                password: None,
            };
            let code = load_contract_code(contract_wasm_path)?;

            let gas_limit = 500_000_000;
            let result = super::execute_call(
                &extrinsic_opts,
                requester,
                target_dest,
                0 as u8,
                code,
                0, // value
                gas_limit,
                HexData::default(), // input
            );

            assert_matches!(result, Ok(_));
            Ok(())
        })
    }
}

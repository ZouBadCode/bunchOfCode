fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = "protos";

    let files = &[
        "protos/sui/rpc/v2/ledger_service.proto",
        "protos/sui/rpc/v2/object.proto",
        "protos/sui/rpc/v2/owner.proto",
        "protos/sui/rpc/v2/checkpoint.proto",
        "protos/sui/rpc/v2/executed_transaction.proto",
        "protos/sui/rpc/v2/system_state.proto",
        "protos/sui/rpc/v2/gas_cost_summary.proto",
        "protos/sui/rpc/v2/execution_status.proto",
        "protos/sui/rpc/v2/balance_change.proto",
        "protos/sui/rpc/v2/event.proto",
        "protos/sui/rpc/v2/object_reference.proto",
        "protos/sui/rpc/v2/state_service.proto",
        "protos/sui/rpc/v2/epoch.proto",
        "protos/sui/rpc/v2/protocol_config.proto",
        "protos/sui/rpc/v2/transaction.proto",
        "protos/sui/rpc/v2/signature.proto",
        "protos/sui/rpc/v2/signature_scheme.proto",
        "protos/sui/rpc/v2/jwk.proto",
        "protos/sui/rpc/v2/bcs.proto",
        "protos/sui/rpc/v2/input.proto",
        "protos/sui/rpc/v2/argument.proto",
        // 對應 google.rpc.Status
        "protos/google/rpc/status.proto",
    ];

    tonic_build::configure()
        .compile_protos(files, &[proto_root])?;

    Ok(())
}
use std::error::Error;
use std::process::Command;

const CIRCUIT_NAME: &str = "email";
const BUILD_DIR: &str = "~/zk-email-verify/build/" + CIRCUIT_NAME;

fn run_commands(nonce: u64) -> Result<(), Box<dyn Error>> {
    let input_wallet_path = format!("../circuits/inputs/input_wallet_{}.json", nonce);
    let witness_path = format!("{}/witness_{}.wtns", BUILD_DIR, nonce);
    let proof_path = format!("{}/rapidsnark_proof_{}.json", BUILD_DIR, nonce);
    let public_path = format!("{}/rapidsnark_public_{}.json", BUILD_DIR, nonce);

    let status1 = Command::new("node")
        .arg(format!(
            "{}/{}_js/generate_witness.js",
            BUILD_DIR, CIRCUIT_NAME
        ))
        .arg(format!(
            "{}/{}_js/{}.wasm",
            BUILD_DIR, CIRCUIT_NAME, CIRCUIT_NAME
        ))
        .arg(&input_wallet_path)
        .arg(&witness_path)
        .status()?;

    if !status1.success() {
        return Err(format!("generate_witness.js failed with status: {}", status1).into());
    }

    let status2 = Command::new("~/rapidsnark/build/prover")
        .arg(format!(
            "{}/{}/{}.zkey",
            BUILD_DIR, CIRCUIT_NAME, CIRCUIT_NAME
        ))
        .arg(&witness_path)
        .arg(&proof_path)
        .arg(&public_path)
        .status()?;

    if !status2.success() {
        return Err(format!("prover failed with status: {}", status2).into());
    }

    Ok(())
}

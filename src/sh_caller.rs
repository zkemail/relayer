use std::error::Error;
use std::process::Command;

const CIRCUIT_NAME: &str = "email";
const BUILD_DIR_PREFIX: &str = "~/zk-email-verify/build/";

pub fn run_commands(nonce: u64) -> Result<(), Box<dyn Error>> {
    let build_dir = format!("{}{}", BUILD_DIR_PREFIX, CIRCUIT_NAME);
    let input_wallet_path = format!("../circuits/inputs/input_wallet_{}.json", nonce);
    let witness_path = format!("{}/witness_{}.wtns", build_dir, nonce);
    let zk_email_path = format!("~/zk_email_verify");
    let proof_path = format!("{}/rapidsnark_proof_{}.json", build_dir, nonce);
    let public_path = format!("{}/rapidsnark_public_{}.json", build_dir, nonce);

    let status0 = Command::new("npx tsx")
        .arg(format!("{}/src/scripts/generate_input.ts", zk_email_path))
        .arg(format!("-e ~/wallet_{}.eml", nonce))
        .arg(format!("-n {}", nonce))
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status0.success() {
        return Err(format!("generate_input.ts failed with status: {}", status0).into());
    }

    let status1 = Command::new("node")
        .arg(format!(
            "{}/{}_js/generate_witness.js",
            build_dir, CIRCUIT_NAME
        ))
        .arg(format!(
            "{}/{}_js/{}.wasm",
            build_dir, CIRCUIT_NAME, CIRCUIT_NAME
        ))
        .arg(&input_wallet_path)
        .arg(&witness_path)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status1.success() {
        return Err(format!("generate_witness.js failed with status: {}", status1).into());
    }

    let status2 = Command::new("~/rapidsnark/build/prover")
        .arg(format!(
            "{}/{}/{}.zkey",
            build_dir, CIRCUIT_NAME, CIRCUIT_NAME
        ))
        .arg(&witness_path)
        .arg(&proof_path)
        .arg(&public_path)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status2.success() {
        return Err(format!("prover failed with status: {}", status2).into());
    }

    Ok(())
}

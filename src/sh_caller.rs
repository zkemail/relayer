// use std::error::Error;
// use std::process::Command;

// const CIRCUIT_NAME: &str = "email";
// // const HOME: &str = "/home/ubuntu";
// const HOME: &str = "../";

// // TODO: Deprecate this file, use circom_proofgen.sh now
// pub fn run_commands(nonce: u64, zk_email_path: &String) -> Result<(), Box<dyn Error>> {
//     // These 3 need to exist
//     // let zk_email_path = format!("{}/zk-email-verify", HOME);
//     let build_dir = format!("{}/build/{}", zk_email_path, CIRCUIT_NAME);
//     let wallet_eml_path = format!("{}/wallet_{}.eml", zk_email_path, nonce);

//     // These s4 will be generated
//     let input_wallet_path = format!("{}/input_wallet_{}.json", HOME, nonce);
//     let witness_path = format!("{}/witness_{}.wtns", build_dir, nonce);
//     let proof_path = format!("{}/rapidsnark_proof_{}.json", build_dir, nonce);
//     let public_path = format!("{}/rapidsnark_public_{}.json", build_dir, nonce);

//     println!(
//         "npx tsx {}/src/scripts/generate_input.ts -e {} -n {}",
//         zk_email_path, wallet_eml_path, nonce
//     );

//     let status0 = Command::new("npx")
//         .arg("tsx")
//         .arg(format!("{}/src/scripts/generate_input.ts", zk_email_path))
//         .arg("-e")
//         .arg(format!("{}/wallet_{}.eml", zk_email_path, nonce))
//         .arg("-n")
//         .arg(format!("{}", nonce))
//         .stdout(std::process::Stdio::inherit())
//         .stderr(std::process::Stdio::inherit())
//         .status()?;

//     println!("status0: {:?}", status0); // Add this line for debugging

//     if !status0.success() {
//         return Err(format!("generate_input.ts failed with status: {}", status0).into());
//     }

//     // TODO: Change to C via https://hackmd.io/V-7Aal05Tiy-ozmzTGBYPA?view#Compilation-and-proving
//     let status1 = Command::new("node")
//         .arg(format!(
//             "{}/{}_js/generate_witness.js",
//             build_dir, CIRCUIT_NAME
//         ))
//         .arg(format!(
//             "{}/{}_js/{}.wasm",
//             build_dir, CIRCUIT_NAME, CIRCUIT_NAME
//         ))
//         .arg(&input_wallet_path)
//         .arg(&witness_path)
//         .stdout(std::process::Stdio::inherit())
//         .stderr(std::process::Stdio::inherit())
//         .status()?;

//     // TODO: Use this C version instead
//     // let status1 = Command::new(format!(
//     //         "./{}/{}_cpp/{}",
//     //         build_dir, CIRCUIT_NAME, CIRCUIT_NAME
//     //     ))
//     //     .arg(&input_wallet_path)
//     //     .arg(&witness_path)
//     //     .stdout(std::process::Stdio::inherit())
//     //     .stderr(std::process::Stdio::inherit())
//     //     .status()?;

//     println!("status1: {:?}", status1); // Add this line for debugging
//     if !status1.success() {
//         return Err(format!("generate_witness.js failed with status: {}", status1).into());
//     }

//     let status2 = Command::new(format!("{}/rapidsnark/build/prover", HOME))
//         .arg(format!("{}/{}.zkey", build_dir, CIRCUIT_NAME))
//         .arg(&witness_path)
//         .arg(&proof_path)
//         .arg(&public_path)
//         .stdout(std::process::Stdio::inherit())
//         .stderr(std::process::Stdio::inherit())
//         .status()?;

//     if !status2.success() {
//         return Err(format!("prover failed with status: {}", status2).into());
//     }

//     Ok(())
// }

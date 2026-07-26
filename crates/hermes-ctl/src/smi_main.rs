//! Drop-in `nvidia-smi` surface backed by Hermes NVML-compatible state.

use hermes_core::HermesManifold;
use hermes_linux::modules;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("nvidia-smi (Hermes GSP)\n");
        println!("No live GSP session is bound by default.");
        println!("Module: {}", modules::NVIDIA);
        return;
    }
    if args.iter().any(|a| a == "-L" || a == "--list-gpus") {
        println!("No devices found (Hermes offline / no bound GPU).");
        return;
    }

    println!("Fri Jul 26 00:00:00 2026");
    println!("+-----------------------------------------------------------------------------+\n| NVIDIA-SMI Hermes-GSP  Driver Version: Hermes-GSP 0.1.0   CUDA Version: N/A |\n+-------------------------------+----------------------+----------------------+\n| GPU  Name        Persistence-M| Bus-Id        Disp.A | Volatile Uncorr. ECC |\n| Fan  Temp  Perf  Pwr:Usage/Cap|         Memory-Usage | GPU-Util  Compute M. |\n|                               |                      |               MIG M. |\n|===============================+======================+======================|\n| No devices were found                                                         |\n+-------------------------------+----------------------+----------------------+");
    let dark = HermesManifold::dark(0);
    assert!(!dark.is_online());
    let _ = dark;
}

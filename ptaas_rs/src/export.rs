use std::path::PathBuf;

use serde_generate::{dart, Encoding, SourceInstaller};
use serde_reflection::{Tracer, TracerConfig};

use crate::models::LocustProject;

pub fn export_models_to_dart(install_dir: PathBuf) {
    let mut tracer = Tracer::new(TracerConfig::default());

    if let Err(err) = tracer.trace_simple_type::<LocustProject>() {
        eprintln!("Failed to trace: {}", err);
        eprintln!("{}", err.explanation());
        return;
    }

    let registry = match tracer.registry() {
        Ok(registry) => registry,
        Err(err) => {
            eprintln!("Failed to trace: {}", err);
            eprintln!("{}", err.explanation());
            return;
        }
    };

    let config = serde_generate::CodeGeneratorConfig::new("models".to_string())
        .with_encodings(vec![Encoding::Bincode, Encoding::Bcs])
        .with_serialization(true);

    let dart_generator = serde_generate::dart::CodeGenerator::new(&config);
    std::fs::create_dir_all(&install_dir).expect("Failed to create dart folder");
    dart_generator
        .output(install_dir.clone(), &registry)
        .expect("Failed to generate dart code");

    let dart_installer = serde_generate::dart::Installer::new(install_dir);
    dart_installer
        .install_module(&config, &registry)
        .expect("Failed to install dart code");
    dart_installer
        .install_serde_runtime()
        .expect("Failed to install dart serde runtime");
    dart_installer
        .install_bincode_runtime()
        .expect("Failed to install dart bincode runtime");
    dart_installer
        .install_bcs_runtime()
        .expect("Failed to install dart bcs runtime");
}

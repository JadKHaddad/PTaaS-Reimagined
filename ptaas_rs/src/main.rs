use ptaas_rs::export::{dummy, export_models_to_dart};

fn main() {
    export_models_to_dart("dart".into());
    dummy();
}

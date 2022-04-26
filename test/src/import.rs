use crate::debug;
use crate::pages;
use crate::short_circuit;
use crate::test_process_string;
use crate::test_process_struct;
use crate::runtime_error;
use holochain_wasmer_host::prelude::*;

pub fn memory_only(store: &Store, env: &Env) -> ImportObject {
    imports! {
        "env" => {
            "__import_data" => Function::new_native_with_env(
                store,
                env.to_owned(),
                holochain_wasmer_host::import::__import_data
            ),
        },
    }
}

pub fn import_object(store: &Store, env: &Env) -> ImportObject {
    imports! {
        "env" => {
            "__import_data" => Function::new_native_with_env(
                store,
                env.clone(),
                holochain_wasmer_host::import::__import_data
            ),
            "__runtime_error" => Function::new_native_with_env(
                store,
                env.clone(),
                runtime_error
            ),
            "__short_circuit" => Function::new_native_with_env(
                store,
                env.clone(),
                short_circuit
            ),
            "__test_process_string" => Function::new_native_with_env(
                store,
                env.clone(),
                test_process_string
            ),
            "__test_process_struct" => Function::new_native_with_env(
                store,
                env.clone(),
                test_process_struct
            ),
            "__debug" => Function::new_native_with_env(
                store,
                env.clone(),
                debug
            ),
            "__pages" => Function::new_native_with_env(
                store,
                env.clone(),
                pages
            ),
        },
    }
}

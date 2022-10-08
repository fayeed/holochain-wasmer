pub mod import;
pub mod wasms;

use holochain_wasmer_host::prelude::*;
use test_common::SomeStruct;

pub fn short_circuit(_: &Env, _: GuestPtr, _: Len) -> Result<u64, wasmer_engine::RuntimeError> {
    Err(wasm_error!(WasmErrorInner::HostShortCircuit(
        holochain_serialized_bytes::encode(&String::from("shorts"))
            .map_err(|e| wasm_error!(e.into()))?,
    ))
    .into())
}

pub fn test_process_string(
    env: &Env,
    guest_ptr: GuestPtr,
    len: Len,
) -> Result<u64, wasmer_engine::RuntimeError> {
    let string: String = env.consume_bytes_from_guest(guest_ptr, len)?;
    let processed_string = format!("host: {}", string);
    Ok(env.move_data_to_guest(Ok::<String, WasmError>(processed_string))?)
}

pub fn test_process_struct(
    env: &Env,
    guest_ptr: GuestPtr,
    len: Len,
) -> Result<u64, wasmer_engine::RuntimeError> {
    let mut some_struct: SomeStruct = env.consume_bytes_from_guest(guest_ptr, len)?;
    some_struct.process();
    Ok(env.move_data_to_guest(Ok::<SomeStruct, WasmError>(some_struct))?)
}

pub fn debug(env: &Env, some_number: WasmSize) -> Result<u64, wasmer_engine::RuntimeError> {
    println!("debug {:?}", some_number);
    Ok(env.move_data_to_guest(())?)
}

pub fn pages(env: &Env, _: WasmSize) -> Result<WasmSize, wasmer_engine::RuntimeError> {
    Ok(env
        .memory_ref()
        .ok_or(wasm_error!(WasmErrorInner::Memory))?
        .size()
        .0)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::wasms;
    use test_common::StringType;
    use wasms::TestWasm;

    #[ctor::ctor]
    fn before() {
        env_logger::init();
    }

    #[test]
    fn infinite_loop() {
        // Instead of looping forever we want the metering to kick in and trap
        // the execution into an unreachable error.
        let result: Result<(), _> = guest::call(TestWasm::Test.instance(), "loop_forever", ());
        assert!(result.is_err());
    }

    #[test]
    fn short_circuit() {
        let result: String = guest::call(TestWasm::Test.instance(), "short_circuit", ()).unwrap();
        assert_eq!(result, String::from("shorts"));
    }

    #[test]
    fn bytes_round_trip() {
        let _: () = guest::call(TestWasm::Memory.instance(), "bytes_round_trip", ()).unwrap();
    }

    #[test]
    fn stacked_test() {
        let result: String = guest::call(TestWasm::Test.instance(), "stacked_strings", ())
            .expect("stacked strings call");

        assert_eq!("first", &result);
    }

    #[test]
    fn literal_bytes() {
        let input: Vec<u8> = vec![1, 2, 3];
        let result: Vec<u8> =
            guest::call(TestWasm::Test.instance(), "literal_bytes", input.clone())
                .expect("literal_bytes call");
        assert_eq!(input, result);
    }

    #[test]
    fn ignore_args_process_string_test() {
        let result: StringType = guest::call(
            TestWasm::Test.instance(),
            "ignore_args_process_string",
            &StringType::from(String::new()),
        )
        .expect("ignore_args_process_string call");
        assert_eq!(String::new(), String::from(result));
    }

    #[test]
    fn process_string_test() {
        // use a "crazy" string that is much longer than a single wasm page to show that pagination
        // and utf-8 are both working OK
        let starter_string = "╰▐ ✖ 〜 ✖ ▐╯".repeat((10_u32 * std::u16::MAX as u32) as _);
        let result: StringType = guest::call(
            TestWasm::Test.instance(),
            "process_string",
            // This is by reference just to show that it can be done as borrowed or owned.
            &StringType::from(starter_string.clone()),
        )
        .expect("process string call");

        let expected_string = format!("host: guest: {}", &starter_string);

        assert_eq!(&String::from(result), &expected_string,);
    }

    #[test]
    fn native_test() {
        let some_inner = "foo";
        let some_struct = SomeStruct::new(some_inner.into());

        let result: SomeStruct = guest::call(
            TestWasm::Test.instance(),
            "native_type",
            some_struct.clone(),
        )
        .expect("native type handling");

        assert_eq!(some_struct, result);
    }

    #[test]
    fn native_struct_test() {
        let some_inner = "foo";
        let some_struct = SomeStruct::new(some_inner.into());

        let result: SomeStruct = guest::call(
            TestWasm::Test.instance(),
            "process_native",
            some_struct.clone(),
        )
        .unwrap();

        let expected = SomeStruct::new(format!("processed: {}", some_inner));
        assert_eq!(result, expected,);
    }

    #[test]
    fn ret_test() {
        let some_struct: SomeStruct =
            guest::call(TestWasm::Test.instance(), "some_ret", ()).unwrap();
        assert_eq!(SomeStruct::new("foo".into()), some_struct,);

        let err: Result<SomeStruct, wasmer_engine::RuntimeError> =
            guest::call(TestWasm::Test.instance(), "some_ret_err", ());
        match err {
            Err(runtime_error) => assert_eq!(
                WasmError {
                    file: "src/wasm.rs".into(),
                    line: 103,
                    error: WasmErrorInner::Guest("oh no!".into()),
                },
                runtime_error.downcast().unwrap(),
            ),
            Ok(_) => unreachable!(),
        };
    }

    #[test]
    fn try_ptr_test() {
        let success_result: Result<SomeStruct, ()> =
            guest::call(TestWasm::Test.instance(), "try_ptr_succeeds", ()).unwrap();
        assert_eq!(SomeStruct::new("foo".into()), success_result.unwrap());

        let fail_result: Result<(), wasmer_engine::RuntimeError> =
            guest::call(TestWasm::Test.instance(), "try_ptr_fails_fast", ());

        match fail_result {
            Err(runtime_error) => {
                assert_eq!(
                    WasmError {
                        file: "src/wasm.rs".into(),
                        line: 132,
                        error: WasmErrorInner::Guest("it fails!: ()".into()),
                    },
                    runtime_error.downcast().unwrap(),
                );
            }
            Ok(_) => unreachable!(),
        };
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn io_test() {
        use std::sync::mpsc;
        use std::sync::mpsc::Receiver;
        use std::sync::mpsc::Sender;

        let instance = TestWasm::Io.instance();
        // let mut tasks = vec![];
        let (tx, rx): (Sender<i32>, Receiver<i32>) = mpsc::channel();
        for _n in 0..1000000 {
            let instance_clone = instance.clone();
            let thread_tx = tx.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let result: Result<String, _> = guest::call(
                    // TestWasm::Io.instance(),
                    instance_clone,
                    "string_input_args_echo_ret",
                    ".".repeat(1000),
                );
                if result.is_err() {
                    dbg!(&result);
                    thread_tx.send(1).unwrap();
                } else {
                    thread_tx.send(0).unwrap();
                }
            })
            .await;
            if rx.recv().unwrap() == 1 {
                break;
            }
        }
        // futures::future::join_all(tasks).await;
    }
}

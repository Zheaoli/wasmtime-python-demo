use pyo3::prelude::*;
use pyo3::types::PyTuple;
use wasmtime::*;
use wasmtime_wasi::preview1::{self};
use wasmtime_wasi::WasiCtxBuilder;
fn main() {
    let mut config = Config::new();
    config.max_wasm_stack(16777216);
    match Engine::new(&config) {
        Ok(engine) => {
            let mut linker = Linker::new(&engine);
            preview1::add_to_linker_sync(&mut linker, |t| t).unwrap();
            linker
                .func_wrap("demo", "demo", |a: i32, b: i32| {
                    Python::with_gil(|py| {
                        let fun: Py<PyAny> = PyModule::from_code_bound(
                            py,
                            "def example(*args, **kwargs):
                                return (args[0] + args[1])*11",
                            "",
                            "",
                        )
                        .unwrap()
                        .getattr("example")
                        .unwrap()
                        .into();
                        let args = PyTuple::new_bound(py, &[a, b]);
                        // cast following to int

                        fun.call1(py, args).unwrap().extract::<i32>(py).unwrap()
                    })
                })
                .unwrap();
            linker.allow_unknown_exports(true);
            let mut builder = WasiCtxBuilder::new();
            builder.inherit_stdio();
            builder.env(
                "PYTHONPATH",
                "/cross-build/wasm32-wasi/build/lib.wasi-wasm32-3.14-pydebug",
            );
            builder
                .preopened_dir(
                    "/workspaces/cpython-wasi",
                    "/",
                    wasmtime_wasi::DirPerms::all(),
                    wasmtime_wasi::FilePerms::all(),
                )
                .unwrap();
            builder.args(&["--", "-c", "import demo; print(demo.bar())"]);
            let wasi_ctx = builder.build_p1();
            let mut store = Store::new(&engine, wasi_ctx);
            let module = Module::from_file(
                &engine,
                "/workspaces/cpython-wasi/cross-build/wasm32-wasi/python.wasm",
            )
            .unwrap();
            let instance = linker.instantiate(&mut store, &module).unwrap();
            let run = instance
                .get_typed_func::<(), ()>(&mut store, "_start")
                .unwrap();
            run.call(&mut store, ()).unwrap();
            return;
        }
        Err(e) => {
            println!("Error creating engine: {:?}", e);
            return;
        }
    }
}

use std::env;
use std::path::PathBuf;

fn main() {
    let bindings = bindgen::Builder::default()
        .header_contents(
            "temporary.h",
            "#include <os/log.h>\n#include <os/signpost.h>",
        )
        // Explicitly allowlist functions and variables.
        .allowlist_function("os_log_create")
        .allowlist_function("os_signpost_enabled")
        .allowlist_function("os_signpost_id_generate")
        .allowlist_function("os_signpost_id_make_with_pointer")
        .allowlist_function("_os_signpost_emit_with_name_impl")
        .allowlist_var("__dso_handle")
        .allowlist_var("OS_LOG_CATEGORY_POINTS_OF_INTEREST")
        .allowlist_var("OS_LOG_CATEGORY_DYNAMIC_TRACING")
        .allowlist_var("OS_LOG_CATEGORY_DYNAMIC_STACK_TRACING")
        .generate()
        .expect("Unable to generate bindings for signpost API");

    let out_path =
        PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR environment variable not set"));

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

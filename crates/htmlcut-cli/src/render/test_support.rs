use std::cell::Cell;

thread_local! {
    static JSON_RENDER_FAILURE_OVERRIDE: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn json_render_failure_is_enabled() -> bool {
    JSON_RENDER_FAILURE_OVERRIDE.with(Cell::get)
}

pub(crate) fn with_json_render_failure_for_tests<T>(operation: impl FnOnce() -> T) -> T {
    struct ResetJsonRenderFailure;

    impl Drop for ResetJsonRenderFailure {
        fn drop(&mut self) {
            JSON_RENDER_FAILURE_OVERRIDE.with(|enabled| enabled.set(false));
        }
    }

    JSON_RENDER_FAILURE_OVERRIDE.with(|enabled| enabled.set(true));
    let _reset = ResetJsonRenderFailure;
    operation()
}

//! Optional observer behavior for generic matching, with no production logger configuration.
use super::*;
#[test]
fn optional_debug_observer_receives_complex_matching_events() {
    struct Observer(std::sync::atomic::AtomicBool);
    impl log::Log for Observer {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool {
            true
        }
        fn log(&self, record: &log::Record<'_>) {
            if record
                .args()
                .to_string()
                .starts_with("Matching complex selector")
            {
                self.0.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
        fn flush(&self) {}
    }
    static OBSERVER: Observer = Observer(std::sync::atomic::AtomicBool::new(false));
    log::set_logger(&OBSERVER).unwrap();
    log::set_max_level(log::LevelFilter::Debug);
    let tree = Tree::new(vec![Record {
        name: "p",
        ..Record::default()
    }]);
    let selectors = parse("p");
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    assert!(matches_selector_list(
        &selectors,
        &tree.element(0),
        &mut context
    ));
    assert!(OBSERVER.0.load(std::sync::atomic::Ordering::Relaxed));
}

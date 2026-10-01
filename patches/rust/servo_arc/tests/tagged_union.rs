use super::*;
use std::sync::{
    Arc as Shared,
    atomic::{AtomicUsize, Ordering},
};

#[test]
fn variants_keep_identity_value_and_borrowed_payloads_distinct() {
    let first = ArcUnion::<String, u64>::from_first(Arc::new("first".into()));
    let second = ArcUnion::<String, u64>::from_second(Arc::new(17));
    assert!(first.is_first());
    assert!(!first.is_second());
    assert!(second.is_second());
    assert!(!second.is_first());
    assert_eq!(&*first.as_first().unwrap(), "first");
    assert!(first.as_second().is_none());
    assert!(second.as_first().is_none());
    assert_eq!(*second.as_second().unwrap(), 17);
    let first_clone = first.clone();
    let second_clone = second.clone();
    assert!(ArcUnion::ptr_eq(&first, &first_clone));
    assert!(ArcUnion::ptr_eq(&second, &second_clone));
    assert_eq!(first.ptr(), first_clone.ptr());
    assert_eq!(first, first_clone);
    assert_eq!(second, second_clone);
    assert_ne!(first, second);
    let same_value = ArcUnion::<String, u64>::from_first(Arc::new("first".into()));
    assert_eq!(first, same_value);
    assert!(!ArcUnion::ptr_eq(&first, &same_value));
    assert!(format!("{first:?}").contains("first"));
    assert!(format!("{second:?}").contains("17"));
    assert_eq!(mem::size_of::<ArcUnion<u8, u16>>(), mem::size_of::<usize>());
    assert_eq!(
        mem::size_of::<Option<ArcUnion<u8, u16>>>(),
        mem::size_of::<usize>()
    );
}

struct Probe(Shared<AtomicUsize>);
impl Drop for Probe {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn each_variant_releases_its_payload_once_after_the_last_owner() {
    for second in [false, true] {
        let count = Shared::new(AtomicUsize::new(0));
        let owner = if second {
            ArcUnion::<Probe, Probe>::from_second(Arc::new(Probe(count.clone())))
        } else {
            ArcUnion::from_first(Arc::new(Probe(count.clone())))
        };
        let clone = owner.clone();
        drop(owner);
        assert_eq!(count.load(Ordering::SeqCst), 0);
        drop(clone);
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
}

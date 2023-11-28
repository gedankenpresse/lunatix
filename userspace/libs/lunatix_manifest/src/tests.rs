extern crate std;

use crate::capabilities::CapSpec;
use crate::manifest::LunatixManifest;
use alloc::vec::Vec;

const MANIFEST: &'static str = "
[metadata]
name=test_name
description=test_description

[environment]
cspace_radix=4
stack_size_bytes=4096

[capabilities]
1=cspace,source=self
2=irq,line=5,notify=false
4=memory,min_size_bytes=4096
";

#[test]
fn test_metadata_name() {
    let m = LunatixManifest::from(MANIFEST);
    assert_eq!(m.metadata().unwrap().name().unwrap(), "test_name")
}

#[test]
fn test_metadata_description() {
    let m = LunatixManifest::from(MANIFEST);
    assert_eq!(
        m.metadata().unwrap().description().unwrap(),
        "test_description"
    )
}

#[test]
fn test_environment_cspace_radix() {
    let m = LunatixManifest::from(MANIFEST);
    assert_eq!(m.environment().unwrap().cspace_radix().unwrap(), 4,)
}

#[test]
fn test_environment_stack_size() {
    let m = LunatixManifest::from(MANIFEST);
    assert_eq!(m.environment().unwrap().stack_size_bytes().unwrap(), 4096,)
}

#[test]
fn test_capabilities() {
    let m = LunatixManifest::from(MANIFEST);
    let mut caps = m.capabilities().unwrap();

    let mut cap_spec = caps.next().unwrap();
    assert_eq!(cap_spec.caddr, 1);
    assert_eq!(cap_spec.typ, "cspace");
    assert_eq!(
        cap_spec.args.iter().collect::<Vec<_>>(),
        std::vec![("source", "self")]
    );
    assert_eq!(cap_spec.args.get("source"), Some("self"));

    cap_spec = caps.next().unwrap();
    assert_eq!(cap_spec.caddr, 2);
    assert_eq!(cap_spec.typ, "irq");
    assert_eq!(
        cap_spec.args.iter().collect::<Vec<_>>(),
        std::vec![("line", "5"), ("notify", "false")]
    );
    assert_eq!(cap_spec.args.get("line"), Some("5"));
    assert_eq!(cap_spec.args.get("notify"), Some("false"));

    cap_spec = caps.next().unwrap();
    assert_eq!(cap_spec.caddr, 4);
    assert_eq!(cap_spec.typ, "memory");
    assert_eq!(
        cap_spec.args.iter().collect::<Vec<_>>(),
        std::vec![("min_size_bytes", "4096")]
    );
    assert_eq!(cap_spec.args.get("min_size_bytes"), Some("4096"));

    assert!(caps.next().is_none());
}

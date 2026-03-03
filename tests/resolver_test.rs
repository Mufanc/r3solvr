use r3solvr::{BasicResolver, Query, SymbolResolver};

const TEST_SO_PATH_1: &str = "tests/data/libandroid_runtime.so";
const TEST_SO_PATH_2: &str = "tests/data/libart.so";

#[test]
fn test_resolver_from_file() {
    let resolver = BasicResolver::from_file(TEST_SO_PATH_1);
    assert!(resolver.is_ok(), "Failed to create resolver from file");
}

#[test]
fn test_lookup_symbol() {
    let resolver = BasicResolver::from_file(TEST_SO_PATH_1).expect("Failed to create resolver");
    let result = resolver
        .lookup_symbol("_ZN7android14AndroidRuntime9getJavaVMEv")
        .expect("Failed to lookup symbol");

    assert_eq!(&*result.name, "_ZN7android14AndroidRuntime9getJavaVMEv");
    assert_eq!(result.addr, 0x0eda6c);
    assert_eq!(result.section_index, 15);
    assert!(!result.stripped);

    let section = resolver
        .lookup_section(result.section_index)
        .expect("Failed to lookup section");

    assert_eq!(&*section.name, ".text");
    assert_eq!(section.file_range, Some((0xe0000, 0xe0000 + 0x149830)));
}

#[test]
fn test_lookup_symbol_prefix() {
    let resolver = BasicResolver::from_file(TEST_SO_PATH_2).expect("Failed to create resolver");
    let query = Query::new("_ZN3artL16GetNativeMethodsEP7_JNIEnvP7_jclassP15JNINativeMethodj")
        .with_prefix(true);

    let result = resolver
        .lookup_symbol(query)
        .expect("Failed to lookup symbol");

    assert_eq!(
        &*result.name,
        "_ZN3artL16GetNativeMethodsEP7_JNIEnvP7_jclassP15JNINativeMethodj.__uniq.224004613612541769487030999398936232930"
    );
    assert_eq!(result.addr, 0x736860);
    assert_eq!(result.section_index, 14);
    assert!(!result.stripped);

    let section = resolver
        .lookup_section(result.section_index)
        .expect("Failed to lookup section");

    assert_eq!(&*section.name, ".text");
    assert_eq!(section.file_range, Some((0x200000, 0x200000 + 0x6273c4)));
}

#[test]
fn test_lookup_symbol_debugdata() {
    let resolver = BasicResolver::from_file(TEST_SO_PATH_1).expect("Failed to create resolver");
    let query = Query::new("_ZL14InitializeOncev")
        .with_debugdata(true)
        .with_prefix(true);

    let result = resolver
        .lookup_symbol(query)
        .expect("Failed to lookup symbol");

    assert_eq!(&*result.name, "_ZL14InitializeOncev");
    assert_eq!(result.addr, 0x20f6d4);
    assert_eq!(result.section_index, 15);
    assert!(result.stripped);

    let section = resolver
        .lookup_section(result.section_index)
        .expect("Failed to lookup section");

    assert_eq!(&*section.name, ".text");
    assert_eq!(section.file_range, Some((0xe0000, 0xe0000 + 0x149830)));
}

#[test]
fn test_lookup_symbol_prefix_debugdata() {
    let resolver = BasicResolver::from_file(TEST_SO_PATH_1).expect("Failed to create resolver");
    let query = Query::new("_ZN12_GLOBAL__N_116SpecializeCommonE")
        .with_debugdata(true)
        .with_prefix(true);

    let result = resolver
        .lookup_symbol(query)
        .expect("Failed to lookup symbol");

    assert_eq!(
        &*result.name,
        "_ZN12_GLOBAL__N_116SpecializeCommonEP7_JNIEnvjjP10_jintArrayiP13_jobjectArraylliP8_jstringS7_bbS7_S7_bS5_S5_bb"
    );
    assert_eq!(result.addr, 0x1f8314);
    assert_eq!(result.section_index, 15);
    assert!(result.stripped);

    let section = resolver
        .lookup_section(result.section_index)
        .expect("Failed to lookup section");

    assert_eq!(&*section.name, ".text");
    assert_eq!(section.file_range, Some((0xe0000, 0xe0000 + 0x149830)));
}

#[test]
fn test_lookup_symbol_not_found() {
    let resolver = BasicResolver::from_file(TEST_SO_PATH_2).expect("Failed to create resolver");
    let result = resolver.lookup_symbol("__nonexistent_symbol_12345__");

    assert!(result.is_err(), "Should not find nonexistent symbol");
}

#[test]
fn test_list_symbols() {
    let resolver = BasicResolver::from_file(TEST_SO_PATH_1).expect("Failed to create resolver");

    for from_list in resolver.list_symbols(true) {
        let query = Query::new(&from_list.name).with_debugdata(true);
        let from_query = resolver.lookup_symbol(query).expect("Failed to lookup symbol");

        assert_eq!(from_list.name, from_query.name);
        // assert_eq!(from_list.addr, from_query.addr);
        assert_eq!(from_list.section_index, from_query.section_index);
        assert_eq!(from_list.stripped, from_query.stripped);
    }
}

#[test]
fn test_lookup_section() {
    let resolver = BasicResolver::from_file(TEST_SO_PATH_2).expect("Failed to create resolver");
    let mut section = resolver.lookup_section(1).expect("Failed to lookup section 1");

    assert_eq!(&*section.name, ".note.android.ident");
    assert_eq!(section.addr, 0x270);
    assert_eq!(section.file_range, Some((0x270, 0x270 + 0x18)));

    section = resolver.lookup_section(3).expect("Failed to lookup section 3");
    assert_eq!(&*section.name, ".dynsym");
    assert_eq!(section.addr, 0x2a8);
    assert_eq!(section.file_range, Some((0x2a8, 0x2a8 + 0x23f58)));

    section = resolver.lookup_section(7).expect("Failed to lookup section 7");
    assert_eq!(&*section.name, ".dynstr");
    assert_eq!(section.addr, 0x2fda0);
    assert_eq!(section.file_range, Some((0x2fda0, 0x2fda0 + 0x5f8f1)));

    section = resolver.lookup_section(14).expect("Failed to lookup section 14");
    assert_eq!(&*section.name, ".text");
    assert_eq!(section.addr, 0x200000);
    assert_eq!(section.file_range, Some((0x200000, 0x200000 + 0x6273c4)));

    section = resolver.lookup_section(15).expect("Failed to lookup section 15");
    assert_eq!(&*section.name, ".plt");
    assert_eq!(section.addr, 0x8273d0);
    assert_eq!(section.file_range, Some((0x8273d0, 0x8273d0 + 0x28b0)));

    section = resolver.lookup_section(22).expect("Failed to lookup section 22");
    assert_eq!(&*section.name, ".data");
    assert_eq!(section.addr, 0xc10b88);
    assert_eq!(section.file_range, Some((0xa10b88, 0xa10b88 + 0x2f01)));

    section = resolver.lookup_section(23).expect("Failed to lookup section 23");
    assert_eq!(&*section.name, ".bss");
    assert_eq!(section.addr, 0xc13a90);
    assert_eq!(section.file_range, None);

    section = resolver.lookup_section(25).expect("Failed to lookup section 25");
    assert_eq!(&*section.name, ".symtab");
    assert_eq!(section.addr, 0x0);
    assert_eq!(section.file_range, Some((0xa13b58, 0xa13b58 + 0xc9990)));

    let result = resolver.lookup_section(29);
    assert!(result.is_err(), "Should fail for out of bounds section index");
}

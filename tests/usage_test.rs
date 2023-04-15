use capnp_import::capnp_import;

mod example_capnp {
    capnp_import::capnp_import!(["tests/example.capnp"]);
    capnp_import::capnp_import!(["tests/folder-test"]);
}

#[test]
fn basic_file_test() {
    use example_capnp::person;
}

#[test]
fn folder_test() {
    //use person;
}

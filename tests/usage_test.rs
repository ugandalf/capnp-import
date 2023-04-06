use capnp_import::capnp_import;

#[test]
fn basic_file_test() {
    capnp_import!(["tests/example.capnp"]);
    //use person;
}

#[test]
fn folder_test() {
    //capnp_import!(["tests/folder-test"]);
    //use person;
}

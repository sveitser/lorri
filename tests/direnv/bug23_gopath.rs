
use std::env;
use direnvtestcase::{DirenvTestCase, DirenvValue};

#[test]
fn bug23_gopath() {
    env::set_var("GOPATH", "my-neat-go-path");
    let mut testcase = DirenvTestCase::new("bug23_gopath");
    testcase.evaluate().expect("Failed to build the first time");

    let env = testcase.get_direnv_variables();
    println!("{:#?}", env);
    assert_eq!(env.get_env("GOPATH"), DirenvValue::Value("my-neat-go-path:/tmp/foo/bar"));
}

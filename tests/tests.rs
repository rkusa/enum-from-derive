#[macro_use]
extern crate enum_from_derive;
extern crate compiletest_rs as compiletest;

use std::path::PathBuf;

fn run_mode(mode: &'static str) {
    let mut config = compiletest::Config::default();

    config.mode = mode.parse().expect("Invalid mode");
    config.src_base = PathBuf::from(format!("tests/{}", mode));
    config.link_deps(); // Populate config.target_rustcflags with dependencies on the path

    compiletest::run_tests(&config);
}

#[test]
fn compile_test() {
    run_mode("ui");
}

#[derive(Debug, PartialEq)]
struct TestError;

#[test]
fn simple() {
    #[derive(Debug, PartialEq, Error)]
    enum Error {
        Test(TestError)
    }

    assert_eq!(Error::Test(TestError), Error::from(TestError));
}

#[test]
fn generics() {
    #[derive(Debug, PartialEq, Error)]
    enum Error<'a, T> where T: Send + 'a {
        Test(TestError),
        Other(&'a T)
    }

    assert_eq!(Error::Test::<i32>(TestError), Error::from(TestError));
    assert_eq!(Error::Other(&42), Error::from(&42));
}

#[test]
fn ignore_multiple_field_variants() {
    #[derive(Debug, PartialEq, Error)]
    enum Error {
        Test(TestError),
        #[allow(dead_code)]
        Other(i32, i32)
    }

    assert_eq!(Error::Test(TestError), Error::from(TestError));
}
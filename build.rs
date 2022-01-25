use npm_rs::*;

fn main() -> Result<(), std::io::Error> {
    NpmEnv::default()
        .with_node_env(&NodeEnv::Production)
        .with_env("FOO", "bar")
        .init_env()
        .install(None)
        .run("build")
        .exec()?;
    println!("Oh hello");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=js");
    Ok(())
}
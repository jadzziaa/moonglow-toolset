//! Builds the vendored official NWScript compiler (see `nwscript/README.md`).

fn main() {
    let dir = std::path::Path::new("nwscript");
    let sources = [
        "native/exostring.cpp",
        "native/scriptcompcore.cpp",
        "native/scriptcomplexical.cpp",
        "native/scriptcompparsetree.cpp",
        "native/scriptcompidentspec.cpp",
        "native/scriptcompfinalcode.cpp",
        "compilerapi.cpp",
    ];
    let mut build = cc::Build::new();
    build.cpp(true).std("c++14").include(dir).include(dir.join("native")).warnings(false);
    if build.get_compiler().is_like_msvc() {
        build.flag("/EHsc");
    }
    for s in sources {
        build.file(dir.join(s));
    }
    build.compile("nwnscriptcomp");
    println!("cargo:rerun-if-changed=nwscript");
}

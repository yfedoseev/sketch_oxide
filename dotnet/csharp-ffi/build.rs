fn main() {
    // This build script could be used with csbindgen to automatically generate C# bindings
    // For now, we're using manual P/Invoke declarations in the C# project

    // If csbindgen is used in future, uncomment this pattern:
    // csbindgen::Builder::default()
    //     .inputs(vec!["src/lib.rs"])
    //     .csharp_class_name("SketchOxideNative")
    //     .csharp_namespace("SketchOxide.Native")
    //     .generate_to_file("../SketchOxide/src/Native/SketchOxideNative.g.cs")
    //     .unwrap();

    println!("cargo:rustc-link-search=native=.");
}

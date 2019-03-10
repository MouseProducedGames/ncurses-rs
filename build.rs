extern crate cc;
extern crate pkg_config;

use pkg_config::Library;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn find_library(names: &[&str]) -> Option<Library> {
    for name in names {
        if let Ok(lib) = pkg_config::probe_library(name) {
            return Some(lib);
        }
    }
    None
}

fn main() {
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");

    let wide = cfg!(all(feature = "wide", not(target_os = "macos")));

    let ncurses_lib_names = if wide {
        &["ncursesw5", "ncursesw", "libncursesw5", "libncursesw"]
    } else {
        &["ncurses5", "ncurses", "libncurses5", "libncurses"]
    };
    let ncurses_lib = find_library(ncurses_lib_names);

    if cfg!(feature = "menu") {
        if wide {
            find_library(&["menuw5", "menuw"]);
        } else {
            find_library(&["menu5", "menu"]);
        }
    }

    if cfg!(feature = "panel") {
        if wide {
            find_library(&["panelw5", "panelw"]);
        } else {
            find_library(&["panel5", "panel"]);
        }
    }

    match std::env::var("NCURSES_RS_RUSTC_LINK_LIB") {
        Ok(x) => println!("cargo:rustc-link-lib={}", x),
        _ => if ncurses_lib.is_none() {
            println!("cargo:rustc-link-lib={}", ncurses_lib_names.last().unwrap())
        }
    }

    if let Ok(x) = std::env::var("NCURSES_RS_RUSTC_FLAGS") {
        println!("cargo:rustc-flags={}", x);
    }

    check_chtype_size(&ncurses_lib);
}

fn check_chtype_size(ncurses_lib: &Option<Library>) {
    let out_dir = env::var("OUT_DIR").expect("cannot get OUT_DIR");
    let src = format!("{}", Path::new(&out_dir).join("chtype_size.c").display());
    let bin = format!("{}", Path::new(&out_dir).join("chtype_size").display());
    let binwin = format!("{}", Path::new(&out_dir).join("chtype_size.exe").display());

    let mut fp = File::create(&src).expect(&format!("cannot create {}", src));
    fp.write_all(b"
#include <assert.h>
#include <limits.h>
#include <stdio.h>

#include <ncurses.h>

int main(void)
{
    if (sizeof(chtype)*CHAR_BIT == 64) {
        puts(\"cargo:rustc-cfg=feature=\\\"wide_chtype\\\"\");
    } else {
        /* We only support 32-bit and 64-bit chtype. */
        assert(sizeof(chtype)*CHAR_BIT == 32 && \"unsupported size for chtype\");
    }

    if (NCURSES_MOUSE_VERSION == 1) {
        puts(\"cargo:rustc-cfg=feature=\\\"mouse_v1\\\"\");
    }
    return 0;
}
    ").expect(&format!("cannot write into {}", src));

    let mut build = cc::Build::new();
    if let Some(lib) = ncurses_lib {
        for path in lib.include_paths.iter() {
            build.include(path);
        }
    }
    let compiler = build.try_get_compiler().expect("Failed Build::try_get_compiler");
    let mut command = compiler.to_command();

    if let Ok(x) = std::env::var("NCURSES_RS_CFLAGS") {
      command.args(x.split(" "));
    }

    command.arg("-o").arg(&bin).arg(&src);
    assert!(command.status().expect("compilation failed").success());
    let features = Command::new(&bin).output()
                   .expect(&format!("{} failed", bin));
    print!("{}", String::from_utf8_lossy(&features.stdout));

    std::fs::remove_file(&src).expect(&format!("cannot delete {}", bin));
    if Path::new(&binwin).is_file()
    {
        std::fs::remove_file(&binwin).expect(&format!("cannot delete {}", bin));
    }
    else
    {
        std::fs::remove_file(&bin).expect(&format!("cannot delete {}", bin));
    }
}

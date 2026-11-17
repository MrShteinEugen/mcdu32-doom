// SPDX-License-Identifier: GPL-3.0-only
use std::path::Path;

// Explicit engine source set. New C files require review here.
const ENGINE_SOURCES: &[&str] = &[
    "am_map.c",
    "d_event.c",
    "d_items.c",
    "d_iwad.c",
    "d_loop.c",
    "d_main.c",
    "d_mode.c",
    "d_net.c",
    "doomdef.c",
    "doomgeneric.c",
    "doomstat.c",
    "dstrings.c",
    "dummy.c",
    "f_finale.c",
    "f_wipe.c",
    "g_game.c",
    "hu_lib.c",
    "hu_stuff.c",
    "i_cdmus.c",
    "i_endoom.c",
    "i_input.c",
    "i_joystick.c",
    "i_scale.c",
    "i_sound.c",
    "i_system.c",
    "i_timer.c",
    "i_video.c",
    "info.c",
    "m_argv.c",
    "m_bbox.c",
    "m_cheat.c",
    "m_config.c",
    "m_controls.c",
    "m_fixed.c",
    "m_menu.c",
    "m_misc.c",
    "m_random.c",
    "memio.c",
    "p_ceilng.c",
    "p_doors.c",
    "p_enemy.c",
    "p_floor.c",
    "p_inter.c",
    "p_lights.c",
    "p_map.c",
    "p_maputl.c",
    "p_mobj.c",
    "p_plats.c",
    "p_pspr.c",
    "p_saveg.c",
    "p_setup.c",
    "p_sight.c",
    "p_spec.c",
    "p_switch.c",
    "p_telept.c",
    "p_tick.c",
    "p_user.c",
    "r_bsp.c",
    "r_data.c",
    "r_draw.c",
    "r_main.c",
    "r_plane.c",
    "r_segs.c",
    "r_sky.c",
    "r_things.c",
    "s_sound.c",
    "sha1.c",
    "sounds.c",
    "st_lib.c",
    "st_stuff.c",
    "statdump.c",
    "tables.c",
    "v_video.c",
    "w_checksum.c",
    "w_file.c",
    "w_file_stdc.c",
    "w_main.c",
    "w_wad.c",
    "wi_stuff.c",
    "z_zone.c",
];

fn main() {
    let engine = Path::new("vendor/doomgeneric");
    let bridge = Path::new("src/doom/native");
    let mut build = cc::Build::new();
    build
        .include(engine)
        .include(bridge)
        .warnings(false)
        .define("DOOMGENERIC_RESX", "320")
        .define("DOOMGENERIC_RESY", "200")
        .define("_CRT_SECURE_NO_WARNINGS", None);
    // Headers may include other headers; Cargo must track the whole directory.
    println!("cargo:rerun-if-changed={}", engine.display());
    println!("cargo:rerun-if-changed={}", bridge.display());
    for source in ENGINE_SOURCES {
        build.file(engine.join(source));
    }
    build.file(bridge.join("doom_bridge.c"));
    build.compile("doomgeneric");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=advapi32");
        println!("cargo:rustc-link-lib=user32");
    }
}

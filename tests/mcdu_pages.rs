// SPDX-License-Identifier: GPL-3.0-only
use mcdu_doom::{
    display::Profile,
    display::atlas::{Atlas, CHARSET},
    display::render::rasterize_grid,
    mcdu::{Avionics, Page},
    output::capture::save_png,
};

#[test]
fn aircraft_font_serializes_into_two_full_size_device_banks_without_reset_reports() {
    use mcdu_doom::device::font::{Packing, rebuild};
    let atlas = mcdu_doom::mcdu::display::atlas();
    let upload = rebuild(&atlas, 23, 29, Packing::Tight, 200).expect("aircraft font upload");
    assert_eq!(upload.banks.len(), 2);
    assert_eq!(
        upload.banks.iter().map(|bank| bank.id).collect::<Vec<_>>(),
        [5, 6]
    );
    for bank in &upload.banks {
        assert_eq!((bank.width, bank.height), (23, 29));
        assert_eq!(bank.row_bytes, 3);
        assert_eq!(bank.record_bytes, 91);
    }
    assert!(
        upload
            .packets
            .iter()
            .all(|packet| !packet.is_empty() && packet.len() <= 64)
    );
    assert!(
        !upload
            .packets
            .iter()
            .any(|packet| packet[..2] == [0xf0, 0] && packet[3] == 2)
    );
}

#[test]
fn aircraft_pages_use_large_and_small_fbw_banks_on_classic_grid() {
    let atlas = mcdu_doom::mcdu::display::atlas();
    let geometry = Profile::Standard.geometry();
    let cols = geometry.columns as usize;
    let rows = geometry.rows as usize;
    let directory = std::env::var_os("MCDU_PAGES_CAPTURE_DIR").map(std::path::PathBuf::from);
    if let Some(path) = &directory {
        std::fs::create_dir_all(path).expect("capture directory");
    }
    let mut pages = Avionics::default();
    let mut frames = Vec::new();
    for (name, page) in [
        ("mcdu-menu", Page::Menu),
        ("init", Page::InitA),
        ("prog", Page::Prog),
        ("perf", Page::Perf(0)),
        ("rad-nav", Page::RadNav),
    ] {
        pages.select(page);
        frames.push((name, pages.render(cols, rows)));
    }
    let game_atlas = Atlas::universal();
    assert_ne!(atlas.glyphs, game_atlas.glyphs);
    for (name, cells) in frames {
        assert_eq!(cells.len(), geometry.cells());
        let reports =
            mcdu_doom::device::protocol::encode_cells(&atlas, &cells).expect("wire frame");
        assert!((16..=18).contains(&reports.len()));
        assert!(cells.iter().any(|cell| cell.glyph >= 110));
        let lines: Vec<String> = cells
            .chunks(24)
            .map(|row| {
                row.iter()
                    .map(|cell| CHARSET.chars().nth((cell.glyph % 110) as usize).unwrap())
                    .collect()
            })
            .collect();
        assert!(!lines.join("\n").contains("DEMO"));
        if name == "mcdu-menu" {
            assert_eq!(&lines[0], "       MCDU MENU        ");
            assert_eq!(&lines[2], "<FMGC          NAV B/UP>");
            assert_eq!(&lines[4], "<ATSU                   ");
            assert_eq!(&lines[6], "<AIDS                   ");
            assert_eq!(&lines[8], "<CFDS                   ");
            assert_eq!(&lines[10], "<doom                   "); // bold capital ink at lowercase wire addresses
            assert!(cells[240..245].iter().all(|cell| cell.color == 6));
            assert_eq!(
                cells[241].glyph,
                mcdu_doom::mcdu::display::emphasis_glyph('D')
            );
            let bold = &atlas.glyphs[cells[241].glyph as usize];
            let normal = &atlas.glyphs[mcdu_doom::mcdu::display::glyph('D', false) as usize];
            assert!(
                bold.bitmap.iter().map(|b| b.count_ones()).sum::<u32>()
                    > normal.bitmap.iter().map(|b| b.count_ones()).sum::<u32>()
            );
            assert!(!lines.join("\n").contains("GAMES"));
            assert_eq!(
                cells[24 + 17].glyph,
                mcdu_doom::mcdu::display::glyph('S', true)
            );
        }
        if let Some(path) = &directory {
            let mut pixels = vec![0; geometry.width() * geometry.height()];
            rasterize_grid(
                &atlas,
                &cells,
                cols,
                rows,
                geometry.glyph_width as usize,
                geometry.glyph_height as usize,
                &mut pixels,
            );
            save_png(
                &path.join(format!("{name}.png")),
                &pixels,
                geometry.width(),
                geometry.height(),
            )
            .expect("PNG");
        }
    }
}

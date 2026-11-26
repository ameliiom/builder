//! The comforts earned in the Skills tree Building (nodes Builder adds to it), with no code tie to Skills: an unlocked
//! node gives the player a vanilla tag that Builder reads, the tree ships as a data file only
//! Skills reads. Without Skills, the server setting `all_comforts` opens them.
use crate::shape::Mode;

/// Tags `builder.size.<n>`: how many blocks a shape holds (8 without any).
pub const SIZE: &str = "builder.size.";
/// The Wall and Floor modes.
pub const SHAPES: &str = "builder.shapes";
/// Undo the last shape.
pub const UNDO: &str = "builder.undo";
pub const SIZE_START: usize = 8;
pub const SIZE_MAX: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Comforts {
    pub size: usize,
    pub shapes: bool,
    pub undo: bool,
}

impl Comforts {
    pub fn of(tags: &[String], all: bool) -> Self {
        if all {
            return Comforts { size: SIZE_MAX, shapes: true, undo: true };
        }
        let size = tags.iter().filter_map(|t| t.strip_prefix(SIZE)?.parse::<usize>().ok()).fold(SIZE_START, usize::max).min(SIZE_MAX);
        Comforts { size, shapes: tags.iter().any(|t| t == SHAPES), undo: tags.iter().any(|t| t == UNDO) }
    }

    /// The mode after `mode` that this player has (Line always; Wall and Floor with the comfort).
    pub fn next_mode(&self, mode: Mode) -> Mode {
        let mut m = mode.next();
        while matches!(m, Mode::Wall | Mode::Floor) && !self.shapes {
            m = m.next();
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn comforts_read_from_tags() {
        assert_eq!(Comforts::of(&t(&[]), false), Comforts { size: 8, shapes: false, undo: false });
        assert_eq!(Comforts::of(&t(&["builder.size.64", "builder.size.16", "builder.undo"]), false), Comforts { size: 64, shapes: false, undo: true });
        assert_eq!(Comforts::of(&t(&["builder.size.99999"]), false).size, 256);
        assert_eq!(Comforts::of(&t(&[]), true), Comforts { size: 256, shapes: true, undo: true });
    }

    #[test]
    fn locked_modes_are_skipped() {
        let plain = Comforts::of(&t(&[]), false);
        assert_eq!(plain.next_mode(Mode::Off), Mode::Line);
        assert_eq!(plain.next_mode(Mode::Line), Mode::Off);
        let all = Comforts::of(&t(&[]), true);
        assert_eq!(all.next_mode(Mode::Line), Mode::Wall);
    }

    /// Both languages hold the same keys, and every node, perk and mode shown has its line.
    #[test]
    fn every_shown_key_is_in_both_languages() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let keys = |lang: &str| -> Vec<String> {
            let text = std::fs::read_to_string(root.join(format!("assets/builder/lang/{lang}.json"))).unwrap();
            let mut k: Vec<String> = text.lines().filter_map(|l| l.trim().strip_prefix('"')?.split('"').next().map(str::to_owned)).collect();
            k.sort();
            k
        };
        let en = keys("en_us");
        assert_eq!(en, keys("ar_sa"));
        let tree = std::fs::read_to_string(root.join("data/builder/skills/building.json")).unwrap();
        let quoted = |field: &str| -> Vec<String> { tree.split(&format!("\"{field}\"")).skip(1).filter_map(|r| r.split('"').nth(1).map(str::to_owned)).collect() };
        for title in quoted("title") {
            assert!(en.contains(&title), "no line for {title}");
        }
        for tag in quoted("tag") {
            assert!(en.contains(&format!("skills.perk.{tag}")), "no perk line for {tag}");
        }
        for mode in Mode::ALL {
            assert!(en.contains(&format!("builder.mode.{}", mode.key())));
        }
    }

    /// Builder adds to Skills' Building tree and never defines a tree of its own (one meaning,
    /// one tree): the file extends `skills:building`, has no title, and its node ids carry
    /// Builder's prefix so they never meet Building's own.
    #[test]
    fn the_nodes_extend_building_instead_of_a_tree_of_their_own() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/builder/skills");
        let files: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
        assert_eq!(files.len(), 1, "one file, the addition to Building: {files:?}");
        let text = std::fs::read_to_string(&files[0]).unwrap();
        assert!(text.contains("\"extends\": \"skills:building\""));
        assert!(!text.contains("\"title\": \"builder.tree"), "a tree of Builder's own");
        let ids: Vec<&str> = text.split("\"id\"").skip(1).filter_map(|r| r.split('"').nth(1)).collect();
        assert!(!ids.is_empty() && ids.iter().all(|id| id.starts_with("builder_")), "{ids:?}");
    }

    /// The tree file gives exactly the tags Builder reads, and every comfort has a node.
    #[test]
    fn the_shipped_tree_gives_exactly_the_tags_builder_reads() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/builder/skills/building.json");
        let text = std::fs::read_to_string(path).unwrap();
        let given: Vec<String> = text.split("\"tag\"").skip(1).filter_map(|r| r.split('"').nth(1).map(str::to_owned)).collect();
        for tag in &given {
            let read = tag == SHAPES || tag == UNDO || tag.strip_prefix(SIZE).is_some_and(|n| n.parse::<usize>().is_ok());
            assert!(read, "the tree gives {tag}, which Builder never reads");
        }
        assert_eq!(Comforts::of(&given, false), Comforts::of(&[], true), "some comfort has no node, or its node stops short");
    }
}

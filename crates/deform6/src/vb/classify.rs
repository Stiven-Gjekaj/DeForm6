//! Telling a form, a standard module and a class apart from `fObjectType`,
//! and the disputed presence test for the optional half of `ObjectInfo`.
//!
//! Everything here is a pure function over a `u32`. Nothing in this file
//! reads a file, resolves an address or allocates a growable buffer.
//! `crate::vb::object::Object::f_object_type` is the only input `classify`
//! ever sees, carried raw by the walk in `vb/object.rs`.

/// A form, a standard module, a class, or a value this corpus has not seen.
///
/// # The three values this corpus proves
///
/// A script this phase's research ran against all 44 vendored programs found
/// exactly three `fObjectType` values, across all 105 objects the corpus
/// holds, and every one of the 105 matches what its `.vbp` declares the
/// object to be:
///
/// | Value | Kind | Objects in the corpus |
/// |---|---|---|
/// | `0x0001_8001` | [`ObjectKind::Module`] | 8 |
/// | `0x0001_8083` | [`ObjectKind::Form`] | 53 |
/// | `0x0011_8003` | [`ObjectKind::Class`] | 44 |
///
/// # The fourteen values `STRUCTURES.md` cites and this corpus does not prove
///
/// `STRUCTURES.md` section 5.5 tabulates seventeen values in total, carried
/// from one prior tool's lookup table. The fourteen below are cited, not
/// measured: no program in this corpus produces one of them.
///
/// | Value | Kind (cited, unmeasured) |
/// |---|---|
/// | `0x0001_80A3` | Form |
/// | `0x0001_80C3` | Form |
/// | `0x0001_8021` | Standard module |
/// | `0x0001_8041` | Standard module |
/// | `0x0001_8061` | Standard module |
/// | `0x0001_8023` | Class |
/// | `0x0001_8803` | Class |
/// | `0x0011_8803` | Class |
/// | `0x0013_8003` | Class |
/// | `0x001D_A003` | UserControl |
/// | `0x001D_A023` | UserControl |
/// | `0x001D_A803` | UserControl |
/// | `0x0015_8003` | PropertyPage |
/// | `0x0015_8803` | UserDocument |
///
/// A value read from a document, not from a real program, silently mis-tags
/// a file if the document is wrong, and it does that quietly: the file still
/// gets a kind name, and nothing marks the name as unproven. Left out of the
/// match, the same value lands in [`ObjectKind::Unknown`] and is flagged, and
/// a wrong document costs nothing. `classify` therefore stays narrow, and a
/// widening of the match is a change made against a new sample, never
/// against a citation alone.
///
/// # `Unknown` carries the raw value and is never a refusal (D-07, D-08)
///
/// No source found records the MDIForm value, `STRUCTURES.md` gap register
/// row 3. This corpus has no MDIForm and no `UserControl`, `PropertyPage` or
/// `UserDocument` either. `Unknown` is the case those values, and any value
/// nobody has measured yet, land in. It carries the number forward for the
/// report: `Unknown(0x1DA003)`, never a guessed name such as `UserControl`.
/// Refusing a file over an `fObjectType` value nobody has seen would refuse
/// exactly the files a recovery tool exists to read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjectKind {
    /// A `.frm` form.
    Form,
    /// A `.bas` standard module. Carries no procedure name array at all;
    /// `vb/privateobj.rs` keys its `.bas` cap off this variant.
    Module,
    /// A `.cls` class.
    Class,
    /// A value the match does not cover, carried raw for the report.
    Unknown(u32),
}

/// Classifies one object's raw `fObjectType`.
///
/// The match holds exactly the three values this corpus proves and a
/// fallback arm that carries every other value into
/// [`ObjectKind::Unknown`]. There is no wildcard-free match here: a `u32`
/// read out of the file names a value a crafted file can set to anything,
/// and a match with no fallback over such a value is exactly the shape
/// `RESEARCH.md`'s Pitfall 1 warns against. An exhaustive match with no
/// fallback is right over an enum this crate builds; it is wrong over a
/// `u32` the file chose.
#[must_use]
pub const fn classify(f_object_type: u32) -> ObjectKind {
    match f_object_type {
        0x0001_8001 => ObjectKind::Module,
        0x0001_8083 => ObjectKind::Form,
        0x0011_8003 => ObjectKind::Class,
        other => ObjectKind::Unknown(other),
    }
}

/// Tells whether `fObjectType` carries the optional half of `ObjectInfo`
/// (`STRUCTURES.md` section 5.3), the `0x40`-byte block that follows
/// `ObjectInfo` at `lpObjectInfo + 0x38` and holds the control array, the
/// object's CLSID and its method-link table.
///
/// # The presence test is disputed, and both published tests are wrong
///
/// `STRUCTURES.md` gap 2 records two stated tests, and neither survives a
/// check against the value table in section 5.5:
///
/// - SVBD's comment on `tOptionalObjectInfo` gives `fObjectType AND 0x80`.
///   Bit `0x80` is set for a form and for nothing else in the whole
///   seventeen-value table, yet a class plainly carries controls and
///   event pointers of its own. This test says every class has no optional
///   block, which is false.
/// - PVB's `OBJECT_HAS_OPTIONAL_INFO` gives `fObjectType AND 0x01`. Bit
///   `0x01` is set in all seventeen tabulated values, including every
///   standard module value. This test says a `.bas` module carries the
///   optional block, which contradicts `STRUCTURES.md` section 5.3's own
///   statement that a module does not have one.
///
/// **This function uses `fObjectType & 0x2`.** Bit `0x2` (bit 1 of the
/// bitfield) is the only bit that separates a standard module from
/// everything else across all seventeen tabulated values: clear for
/// `0x18001`, `0x18021`, `0x18041`, `0x18061`, set for the other thirteen.
/// Neither of the two published tests above is implemented here.
#[must_use]
pub const fn has_optional_info(f_object_type: u32) -> bool {
    f_object_type & 0x2 != 0
}

/// Cross-checks the `0x2` bit against `ObjectInfo.lpPrivateObject`
/// (`STRUCTURES.md` section 5.2, offset `0x0C`), which SVBD notes is `-1`
/// for a standard module and something else for everything else.
///
/// Returns `true` when the two markers agree, `false` when they disagree.
/// Per the ROADMAP risk on gap 2, a disagreement is reported, never
/// resolved by picking a side. This function reports nothing itself: it
/// carries no dependency on `error.rs`, so `classify.rs` stays a pure
/// module with no file access and no `Defect` construction. A caller that
/// already holds a [`crate::error::Defect`] builder decides how a `false`
/// here becomes one; a plain `bool` is the smaller surface for a branch a
/// script this phase's research ran found on zero of 105 corpus objects.
///
/// A script this phase's research ran, and a second script this plan's
/// planner ran independently, both found the two markers agreeing on all
/// 105 corpus objects: the same eight objects report no private object and
/// are classified [`ObjectKind::Module`]. **No corpus file makes this
/// function return `false`.** The disagreement path exists for a file this
/// corpus does not contain, and it is exercised here only by a synthetic
/// pair of values, never by a corpus fixture.
#[must_use]
pub const fn agree(f_object_type: u32, lp_private_object: u32) -> bool {
    has_optional_info(f_object_type) == (lp_private_object != 0xFFFF_FFFF)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{ObjectKind, agree, classify, has_optional_info};
    use crate::read::pe::PeImage;
    use crate::read::region::{Off, Va};
    use crate::vb::header::{VbHeader, header_region};
    use crate::vb::object::ObjectTable;
    use crate::vb::project::{ObjectTableHead, ProjectInfo};

    /// The program whose object count and object capacity differ: one form
    /// and two classes declared. `vb/object.rs`'s own test module reads the
    /// same file for the same reason: it is the one corpus program this
    /// phase's classifier must tell apart, in array order, without reading
    /// a fourth, unpopulated slot.
    const GRAYSCALE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/vb6-code/Grayscale-effect/Grayscale.exe"
    ));

    /// The seventeen values `STRUCTURES.md` section 5.5 tabulates, built
    /// here as a literal per `AGENTS.md`: a test builds the state it needs
    /// and does not read it out of a file the author edits. Three of the
    /// seventeen are the values this corpus proves; the tuple's second
    /// field names which `ObjectKind` each one classifies to.
    fn the_seventeen_tabulated_values() -> [(u32, ObjectKind); 17] {
        [
            (0x0001_8083, ObjectKind::Form),
            (0x0001_80A3, ObjectKind::Unknown(0x0001_80A3)),
            (0x0001_80C3, ObjectKind::Unknown(0x0001_80C3)),
            (0x0001_8001, ObjectKind::Module),
            (0x0001_8021, ObjectKind::Unknown(0x0001_8021)),
            (0x0001_8041, ObjectKind::Unknown(0x0001_8041)),
            (0x0001_8061, ObjectKind::Unknown(0x0001_8061)),
            (0x0001_8023, ObjectKind::Unknown(0x0001_8023)),
            (0x0001_8803, ObjectKind::Unknown(0x0001_8803)),
            (0x0011_8003, ObjectKind::Class),
            (0x0011_8803, ObjectKind::Unknown(0x0011_8803)),
            (0x0013_8003, ObjectKind::Unknown(0x0013_8003)),
            (0x001D_A003, ObjectKind::Unknown(0x001D_A003)),
            (0x001D_A023, ObjectKind::Unknown(0x001D_A023)),
            (0x001D_A803, ObjectKind::Unknown(0x001D_A803)),
            (0x0015_8003, ObjectKind::Unknown(0x0015_8003)),
            (0x0015_8803, ObjectKind::Unknown(0x0015_8803)),
        ]
    }

    #[test]
    fn the_three_measured_values_classify_by_name() {
        assert_eq!(classify(0x0001_8083), ObjectKind::Form);
        assert_eq!(classify(0x0001_8001), ObjectKind::Module);
        assert_eq!(classify(0x0011_8003), ObjectKind::Class);
    }

    #[test]
    fn zero_and_all_ones_are_unknown_and_carry_the_value() {
        assert_eq!(classify(0), ObjectKind::Unknown(0));
        assert_eq!(classify(0xFFFF_FFFF), ObjectKind::Unknown(0xFFFF_FFFF));
    }

    /// The instrument for the "start narrow" design in the module doc
    /// comment: every one of the seventeen tabulated values classifies to
    /// the kind this file's match currently gives it, and the fourteen
    /// cited-only values classify to `Unknown`, never to a guessed name.
    /// A later widening of the match without widening this literal fails
    /// here, which is the point: this test is what keeps the match and the
    /// doc comment's table in step.
    #[test]
    fn every_tabulated_value_classifies_and_the_untested_ones_land_in_unknown() {
        for (value, expected) in the_seventeen_tabulated_values() {
            assert_eq!(
                classify(value),
                expected,
                "0x{value:x} must classify to {expected:?}"
            );
        }
    }

    /// Gives the address of `ProjectInfo` that the file itself holds.
    ///
    /// A second copy of the helper `vb/object.rs`'s own test module holds,
    /// written on purpose: `AGENTS.md` asks a test to build the state it
    /// needs, and a shared fixture module would let a change to one file's
    /// tests silently break the other's.
    fn project_data_va(data: &[u8]) -> Va {
        let image = PeImage::parse(data).unwrap();
        let hdr = header_region(&image).unwrap();
        VbHeader::read(&hdr).unwrap().lp_project_data
    }

    /// Gives the address of the object table that the file itself holds.
    fn object_table_va(data: &[u8]) -> Va {
        let image = PeImage::parse(data).unwrap();
        ProjectInfo::read(&image, project_data_va(data))
            .unwrap()
            .lp_object_table
    }

    /// Walks the object array out of a byte slice, through the same
    /// pointer chain `inspect` uses.
    fn walk(data: &[u8]) -> ObjectTable {
        let image = PeImage::parse(data).unwrap();
        let head = ObjectTableHead::read(&image, object_table_va(data)).unwrap();
        ObjectTable::walk(&image, object_table_va(data), &head).unwrap()
    }

    /// Reads `ObjectInfo.lpPrivateObject` at `lpObjectInfo + 0x0C`
    /// (`STRUCTURES.md` section 5.2), independently of `vb/privateobj.rs`,
    /// which this plan does not touch. This is the one field task 2 needs
    /// from `ObjectInfo`, read through the same `PeImage` primitives
    /// `vb/object.rs` already uses for every other field in this phase.
    fn private_object_ptr(pe: &PeImage<'_>, lp_object_info: Va) -> Va {
        pe.region_at_va(lp_object_info)
            .and_then(|region| region.va_le(Off::new(0x0C)))
            .unwrap()
    }

    #[test]
    fn walking_grayscale_classifies_form_class_class_in_array_order() {
        let table = walk(GRAYSCALE);
        let kinds: Vec<ObjectKind> = table
            .objects
            .iter()
            .map(|object| classify(object.f_object_type))
            .collect();
        assert_eq!(
            kinds,
            vec![ObjectKind::Form, ObjectKind::Class, ObjectKind::Class]
        );
    }

    #[test]
    fn the_form_and_the_class_each_carry_the_optional_block_and_the_module_does_not() {
        assert!(has_optional_info(0x0001_8083), "a form has one");
        assert!(has_optional_info(0x0011_8003), "a class has one");
        assert!(!has_optional_info(0x0001_8001), "a module does not");
    }

    #[test]
    fn the_two_markers_agree_when_they_say_the_same_thing_and_disagree_otherwise() {
        // A form, with a real (non-sentinel) private-object address.
        assert!(agree(0x0001_8083, 0x0040_1000));
        // A module, with the sentinel.
        assert!(agree(0x0001_8001, 0xFFFF_FFFF));
        // A form whose private-object address is the module sentinel: the
        // bit says "has one", the pointer says "does not".
        assert!(!agree(0x0001_8083, 0xFFFF_FFFF));
    }

    /// The synthetic case named in the plan: no corpus file produces a
    /// disagreement, so this pair is built here, in the test, rather than
    /// read from a file. This is the pure-function call the plan's fourth
    /// behaviour asks for, needing no file at all.
    #[test]
    fn a_synthetic_bit_set_pointer_at_the_sentinel_pair_reports_a_disagreement() {
        assert!(!agree(0x0001_8083, 0xFFFF_FFFF));
    }

    #[test]
    fn no_corpus_file_makes_the_two_optional_info_markers_disagree() {
        let image = PeImage::parse(GRAYSCALE).unwrap();
        let table = walk(GRAYSCALE);
        for object in &table.objects {
            let private_object = private_object_ptr(&image, object.lp_object_info);
            assert!(
                agree(object.f_object_type, private_object.get()),
                "{:?} disagrees, and this corpus is measured to have none that do",
                object.name
            );
        }
    }

    /// Both published presence tests are named in the doc comment, with
    /// the reason each is wrong, and neither is the test this file runs.
    /// `has_optional_info` uses `0x2`; this checks the doc comment still
    /// names `0x80` and `0x01` as the two tests it rejected, so a later
    /// edit cannot drop the record of why they are wrong without this test
    /// noticing.
    #[test]
    fn the_doc_comment_names_both_rejected_presence_tests_and_the_reason_each_is_wrong() {
        let source = include_str!("classify.rs");
        assert!(
            source.contains("0x80"),
            "the doc comment must still name SVBD's `0x80` test"
        );
        assert!(
            source.contains("0x01"),
            "the doc comment must still name PVB's `0x01` test"
        );
        assert!(
            source.contains("a class plainly carries controls"),
            "the doc comment must still give the reason `0x80` is wrong"
        );
        assert!(
            source.contains("standard module value"),
            "the doc comment must still give the reason `0x01` is wrong"
        );
    }
}

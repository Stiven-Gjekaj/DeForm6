//! Telling a form, a standard module and a class apart from `fObjectType`.
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test builds its own literal; a wrong value must fail loudly"
)]
mod tests {
    use super::{ObjectKind, classify};
    use crate::read::pe::PeImage;
    use crate::read::region::Va;
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
}

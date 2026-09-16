//! The census: for each array this crate walks, the count the file declares
//! against the number of instances the reader returns.
//!
//! # Why a census, and not another byte diff
//!
//! The byte diff can only find a fault where the reader keeps a cooked value
//! in a field that an emitter writes back. Most of this reader's clamps are
//! not that shape. They bound a loop and nothing else: the reader reads fewer
//! elements than the file declares, and no byte anywhere changes. A byte diff
//! is blind to that. A count is not.
//!
//! # What one row says
//!
//! A [`Count`] names an array, the structure and offset its declared count was
//! read from, the count itself, how many instances the reader returned, and
//! the reason the two differ when they do. The reasons are kept apart on
//! purpose. A clamp, an array whose address maps nowhere, and a reader that
//! refused are three different facts about a file, and a row that only said
//! "fewer" would hide which one happened.
//!
//! # A clamp is matched by field name, never by offset
//!
//! A reader's `ImplausibleCount` defect is matched to a row by the name of the
//! count field. It is not matched by the defect's offset. `ImplausibleCount`
//! documents its offset as that of the count field, and three readers were
//! found to record the start of the table there instead, so an offset match
//! would miss real clamps.

use crate::error::{Defect, DefectKind, Refusal};
use crate::read::region::Off;

/// One array the census counts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Array {
    /// The GUI table, `VBHeader.wFormCount` entries long.
    GuiTable,
    /// The object array, `ObjectTable.wTotalObjects` records long.
    Objects,
    /// One object's `ControlInfo` array, `OptionalObjectInfo.dwControlCount`
    /// entries long.
    Controls,
    /// One control's event slots, `ControlInfo.wEventCount` slots long.
    EventSlots,
}

impl Array {
    /// The structure that holds the declared count.
    #[must_use]
    pub const fn structure(self) -> &'static str {
        match self {
            Self::GuiTable => "VBHeader",
            Self::Objects => "ObjectTable",
            Self::Controls => "OptionalObjectInfo",
            Self::EventSlots => "ControlInfo",
        }
    }

    /// The name of the declared count field, spelled as the reader's own
    /// defects spell it.
    #[must_use]
    pub const fn field(self) -> &'static str {
        match self {
            Self::GuiTable => "wFormCount",
            Self::Objects => "wTotalObjects",
            Self::Controls => "dwControlCount",
            Self::EventSlots => "wEventCount",
        }
    }

    /// The width of the declared count field, in bytes.
    #[must_use]
    pub const fn width(self) -> u32 {
        match self {
            Self::GuiTable | Self::Objects | Self::EventSlots => 2,
            Self::Controls => 4,
        }
    }
}

/// What an array belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Owner {
    /// The program as a whole.
    Program,
    /// One form, by its index in the GUI table.
    Form {
        /// The form's index in the GUI table.
        form: u32,
    },
    /// One object, by its index in the object array.
    Object {
        /// The object's index in the object array.
        object: u32,
    },
    /// One control of one object.
    Control {
        /// The owning object's index in the object array.
        object: u32,
        /// The control's index in that object's `ControlInfo` array.
        control: u32,
    },
}

/// Why the declared count and the returned count agree or differ.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    /// The reader returned exactly as many instances as the file declares.
    Whole,
    /// The declared count asks for more than the file can hold, and the
    /// reader bounded its loop to `max`.
    Clamped {
        /// The largest count the file can hold.
        max: u32,
    },
    /// The array's address maps to no section, so the reader read nothing.
    Unmapped,
    /// The reader refused, so it returned nothing.
    Refused(Refusal),
    /// The reader returned nothing because it knows no header layout for
    /// this control type.
    Unsized {
        /// The control type it did not know.
        f_control_type: u16,
    },
    /// The two counts differ for a reason no rule above names.
    ///
    /// This is the outcome to look at first. It covers a reader that returned
    /// more than the file declares.
    Unexplained,
}

/// One array, counted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Count {
    /// The array.
    pub array: Array,
    /// What the array belongs to.
    pub owner: Owner,
    /// The absolute file offset of the declared count field.
    ///
    /// The walk computes this from its own restated layout, never from a
    /// reader's defect.
    pub declared_at: Off,
    /// The count the file declares, as the reader's model holds it.
    pub declared: u32,
    /// The number of instances the reader returned.
    pub recovered: u32,
    /// Why the two agree or differ.
    pub outcome: Outcome,
}

/// What a walk knows about one array when it decides the outcome.
///
/// Grouped into one value so that the decision stays one function with one
/// argument.
pub(crate) struct Evidence<'a> {
    /// The array.
    pub(crate) array: Array,
    /// The count the file declares.
    pub(crate) declared: u32,
    /// The number of instances the reader returned.
    pub(crate) recovered: u32,
    /// Every defect the reader raised while it read this array.
    pub(crate) defects: &'a [Defect],
    /// Whether the walk found the array's address to map nowhere.
    pub(crate) unmapped: bool,
    /// The control type the reader knew no layout for, if any.
    pub(crate) unsupported_control_type: Option<u16>,
}

/// Decides why a reader that did not refuse returned what it returned.
///
/// A refusal is not decided here. The walk records it directly, because a
/// reader that refused returned no instances and raised no defect to read.
///
/// The rules are tried in this order, and the first that holds wins:
///
/// ```text
/// declared == recovered                                      Whole
/// an ImplausibleCount on this field, and recovered == max    Clamped
/// the address maps nowhere, and nothing returned             Unmapped
/// no layout for the control type, and nothing returned       Unsized
/// anything else                                              Unexplained
/// ```
pub(crate) fn outcome(evidence: &Evidence<'_>) -> Outcome {
    if evidence.declared == evidence.recovered {
        return Outcome::Whole;
    }

    for defect in evidence.defects {
        if let DefectKind::ImplausibleCount { max, .. } = &defect.kind
            && defect.site.field == evidence.array.field()
            && evidence.recovered == *max
        {
            return Outcome::Clamped { max: *max };
        }
    }

    if evidence.unmapped && evidence.recovered == 0 {
        return Outcome::Unmapped;
    }

    if let Some(f_control_type) = evidence.unsupported_control_type
        && evidence.recovered == 0
    {
        return Outcome::Unsized { f_control_type };
    }

    Outcome::Unexplained
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::integer_division,
        reason = "a test builds the state it needs and must fail loudly when that state is wrong"
    )]

    use super::{Array, Evidence, Outcome, outcome};
    use crate::error::{Defect, DefectKind, Site};

    fn clamp(field: &'static str, count: u32, max: u32) -> Defect {
        Defect {
            site: Site {
                offset: 0x1000,
                rva: None,
                structure: "Test",
                field,
            },
            kind: DefectKind::ImplausibleCount {
                offset: 0x1000,
                count,
                max,
            },
        }
    }

    fn evidence(array: Array, declared: u32, recovered: u32, defects: &[Defect]) -> Evidence<'_> {
        Evidence {
            array,
            declared,
            recovered,
            defects,
            unmapped: false,
            unsupported_control_type: None,
        }
    }

    #[test]
    fn the_four_arrays_name_the_fields_the_readers_own_defects_name() {
        // These strings are what a clamp is matched on. A one letter drift
        // would turn every real clamp into an unexplained row, and no corpus
        // program clamps, so the corpus would never show it.
        assert_eq!(Array::GuiTable.field(), "wFormCount");
        assert_eq!(Array::Objects.field(), "wTotalObjects");
        assert_eq!(Array::Controls.field(), "dwControlCount");
        assert_eq!(Array::EventSlots.field(), "wEventCount");
    }

    #[test]
    fn each_array_states_the_width_of_its_own_count_field() {
        assert_eq!(Array::GuiTable.width(), 2);
        assert_eq!(Array::Objects.width(), 2);
        assert_eq!(Array::Controls.width(), 4);
        assert_eq!(Array::EventSlots.width(), 2);
    }

    #[test]
    fn equal_counts_are_whole_even_when_a_defect_is_present() {
        let defects = [clamp("wFormCount", 9, 9)];
        assert_eq!(
            outcome(&evidence(Array::GuiTable, 9, 9, &defects)),
            Outcome::Whole
        );
    }

    #[test]
    fn a_clamp_on_the_arrays_own_field_that_matches_what_was_returned_is_clamped() {
        let defects = [clamp("dwControlCount", 1000, 3)];
        assert_eq!(
            outcome(&evidence(Array::Controls, 1000, 3, &defects)),
            Outcome::Clamped { max: 3 }
        );
    }

    #[test]
    fn a_clamp_on_another_field_is_never_taken_for_this_arrays_clamp() {
        // The object walk raises ProcCount clamps. None of them may explain a
        // short object array.
        let defects = [clamp("ProcCount", 40, 2)];
        assert_eq!(
            outcome(&evidence(Array::Objects, 5, 2, &defects)),
            Outcome::Unexplained
        );
    }

    #[test]
    fn a_clamp_whose_maximum_is_not_what_was_returned_does_not_explain_it() {
        let defects = [clamp("wEventCount", 50, 10)];
        assert_eq!(
            outcome(&evidence(Array::EventSlots, 50, 4, &defects)),
            Outcome::Unexplained
        );
    }

    #[test]
    fn an_unmapped_array_that_returned_nothing_is_unmapped() {
        let mut e = evidence(Array::Controls, 7, 0, &[]);
        e.unmapped = true;
        assert_eq!(outcome(&e), Outcome::Unmapped);
    }

    #[test]
    fn an_unmapped_flag_does_not_explain_an_array_that_returned_something() {
        let mut e = evidence(Array::Controls, 7, 2, &[]);
        e.unmapped = true;
        assert_eq!(outcome(&e), Outcome::Unexplained);
    }

    #[test]
    fn an_unknown_control_type_that_returned_nothing_is_unsized() {
        let mut e = evidence(Array::EventSlots, 12, 0, &[]);
        e.unsupported_control_type = Some(0x41);
        assert_eq!(
            outcome(&e),
            Outcome::Unsized {
                f_control_type: 0x41
            }
        );
    }

    #[test]
    fn a_reader_that_returned_more_than_the_file_declares_is_unexplained() {
        assert_eq!(
            outcome(&evidence(Array::GuiTable, 2, 3, &[])),
            Outcome::Unexplained
        );
    }

    #[test]
    fn a_clamp_is_preferred_to_an_unmapped_flag_when_both_would_hold() {
        // The order is part of the contract: a clamp that bounded the loop to
        // zero is a clamp, whatever else the walk noticed.
        let defects = [clamp("dwControlCount", 5, 0)];
        let mut e = evidence(Array::Controls, 5, 0, &defects);
        e.unmapped = true;
        assert_eq!(outcome(&e), Outcome::Clamped { max: 0 });
    }
}
